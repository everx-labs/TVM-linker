/*
 * Copyright 2018-2022 TON DEV SOLUTIONS LTD.
 *
 * Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
 * this file except in compliance with the License.
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific TON DEV software governing permissions and
 * limitations under the License.
 */
use crate::methdict::*;
use crate::parser::{ptr_to_builder, ParseEngine, ParseEngineResults, SelectorVariant};
use crate::printer::tree_of_cells_into_base64;
use ed25519_dalek::*;
use ever_struct::scheme;
use failure::format_err;
use serde_json::json;
use std::collections::HashMap;
use std::fs::File;
use std::io::Cursor;
use std::io::Read;
use std::time::SystemTime;
use ton_block::*;
use ton_labs_assembler::{compile_code_debuggable, DbgInfo, Line, Lines};
use ton_types::deserialize_cells_tree;
use ton_types::deserialize_cells_tree_ex;
use ton_types::dictionary::{HashmapE, HashmapType};
use ton_types::{BuilderData, Cell, IBitstring, Result, SliceData};

pub struct Program {
    language: Option<String>,
    engine: ParseEngineResults,
    keypair: Option<Keypair>,
    pub dbgmap: DbgInfo,
    print_code: bool,
    silent: bool,
}

impl Program {
    pub fn new(parser: ParseEngine) -> Self {
        Program {
            language: None,
            engine: ParseEngineResults::new(parser),
            keypair: None,
            dbgmap: DbgInfo::default(),
            print_code: false,
            silent: false,
        }
    }

    pub fn set_print_code(&mut self, print_code: bool) {
        self.print_code = print_code;
    }

    pub fn set_silent(&mut self, silent: bool) {
        self.silent = silent;
    }

    pub fn set_language(&mut self, lang: Option<&str>) {
        self.language = lang.map(|s| s.to_owned());
    }

    pub fn data(&self) -> Result<Cell> {
        let bytes =
            if let Some(ref pair) = self.keypair {
                pair.public.to_bytes()
            } else {
                [0u8; PUBLIC_KEY_LENGTH]
            };

        // Persistent data feature is obsolete and should be removed.
        // Off-chain constructor should be used to create data layout instead.
        let (persistent_base, persistent_data) = self.engine.persistent_data();
        let mut data_dict = HashmapE::with_hashmap(64, None);
        if let Some(ref lang) = self.language {
            if lang == "C" || lang == "c" {
                data_dict = HashmapE::with_hashmap(64, persistent_data)
            }
        }
        let key = SliceData::load_builder(ptr_to_builder(persistent_base)?)?;
        let data = BuilderData::with_raw(bytes.to_vec(), PUBLIC_KEY_LENGTH * 8)?;
        data_dict.set(key, &SliceData::load_builder(data)?)
            .map_err(|e| format_err!("failed to pack pubkey to data dictionary: {}", e))?;
        let mut builder = BuilderData::new();
        builder
            .append_bit_one()?
            .checked_append_reference(data_dict.data().unwrap().clone())?;
        builder.into_cell()
    }

    pub fn entry(&self) -> Lines {
        self.engine.entry()
    }

    pub fn internal_method_dict(&mut self) -> Result<Option<Cell>> {
        let mut dict = prepare_methods(&self.engine.privates(), true)
            .map_err(|(i, s)| format_err!("{}", s.replace("_name_", &self.engine.global_name(i).unwrap())))?;
        self.dbgmap.append(&mut dict.1);
        Ok(dict.0.data().cloned())
    }

    fn publics_filtered(&self, remove_ctor: bool) -> HashMap<u32, Lines> {
        self.engine.publics().into_iter()
            .filter(|(k, _)|
                !(remove_ctor && self.engine.global_name(*k).unwrap_or_default() == "constructor")
            ).collect()
    }

    pub fn public_method_dict(&mut self, remove_ctor: bool) -> Result<Option<Cell>> {
        let mut dict = prepare_methods(&self.engine.internals(), true)
            .map_err(|(i, s)| format_err!("{}", s.replace("_name_", &self.engine.internal_name(i).unwrap())))?;

        insert_methods(&mut dict.0, &mut dict.1, &self.publics_filtered(remove_ctor), true)
            .map_err(|(i, s)| format_err!("{}", s.replace("_name_", &self.engine.global_name(i).unwrap())))?;

        self.dbgmap.append(&mut dict.1);

        Ok(dict.0.data().cloned())
    }

    fn wrap_into_tvc(&mut self, code: Cell, no_meta: bool) -> scheme::TVC {
        let meta = if !no_meta {
            let sold = scheme::Version::new(
                self.engine.commith().unwrap_or(&[0u8; 20]).clone(),
                self.engine.version().unwrap_or_default(),
            );

            let linker: [u8; 20] = hex::decode(env!("BUILD_GIT_COMMIT"))
                .unwrap()
                .try_into()
                .unwrap();

            let linker = scheme::Version::new(
                linker,
                env!("CARGO_PKG_VERSION").to_string()
            );

            let compiled_at = get_now() as u64;
            let name = scheme::SmallStr::new(self.engine.name().to_string());
            let desc = self.engine.desc();

            Some(scheme::Metadata::new(sold, linker, compiled_at, name, desc.to_owned()))
        } else {
            None
        };

        scheme::TVC::new(scheme::TvmSmc::TvcFrst(scheme::TvcFrst::new(code, meta)))
    }

    pub fn compile_to_file_ex(
        &mut self,
        prefix: &str,
        out_file: Option<&str>,
        no_meta: bool,
        raw: bool,
    ) -> Result<String> {
        let code = self.compile_asm(false)?;
        let cell = if !raw { self.wrap_into_tvc(code, no_meta).serialize()? } else { code };

        if self.print_code {
            let json = serde_json::to_string_pretty(&json!({
                "base64_boc": tree_of_cells_into_base64(Some(&cell)),
                "raw": raw,
            }))?;

            println!("{}", json);
            return Ok("".to_string());
        }

        let ext = if raw { ".code.boc" } else { ".tvc.boc" };
        let filename = format!("{}{}", prefix, ext);
        let filename = out_file.unwrap_or(&filename);

        cell.write_to_file(filename);
        println!("Contract compiled and saved to: \"{}\"", filename);

        Ok(filename.to_string())
    }

    fn compile_asm_old(&mut self, remove_ctor: bool) -> Result<Cell> {
        let internal_selector_text = vec![
            Line::new("DICTPUSHCONST 32\n", "<internal-selector>", 1),
            Line::new("DICTUGETJMP\n",      "<internal-selector>", 2),
        ];
        let mut internal_selector = compile_code_debuggable(internal_selector_text)
            .map_err(|e| format_err!("unexpected error while compiling internal selector: {}", e))?;
        internal_selector.0.append_reference(SliceData::load_cell(self.internal_method_dict()?.unwrap_or_default())?);

        // adjust hash of internal_selector cell
        let hash = internal_selector.0.cell().repr_hash();
        assert_eq!(internal_selector.1.len(), 1);
        let entry = internal_selector.1.first_entry().unwrap();
        self.dbgmap.insert(hash, entry.clone());

        let (mut main_selector, main_selector_dbg) = compile_code_debuggable(self.entry())
            .map_err(|e| format_err!("unexpected error while compiling main selector: {}", e))?;
        main_selector.append_reference(SliceData::load_cell(self.public_method_dict(remove_ctor)?.unwrap_or_default())?);
        main_selector.append_reference(internal_selector.0);

        // adjust hash of main_selector cell
        let hash = main_selector.cell().repr_hash();
        assert_eq!(main_selector_dbg.len(), 1);
        let entry = main_selector_dbg.first_entry().unwrap();
        self.dbgmap.insert(hash, entry.clone());

        Ok(main_selector.cell().clone())
    }

    pub fn compile_asm(&mut self, remove_ctor: bool) -> Result<Cell> {
        if !self.entry().is_empty() {
            // TODO wipe out the old behavior
            return self.compile_asm_old(remove_ctor);
        }

        let internal_selector_text = vec![
            // indirect jump
            Line::new("DICTPUSHCONST 32\n", "<internal-selector>", 1),
            Line::new("DICTUGETJMPZ\n",     "<internal-selector>", 2),
            Line::new("THROW 78\n",         "<internal-selector>", 3),
        ];

        let mut internal_selector = compile_code_debuggable(internal_selector_text)
            .map_err(|e| format_err!("unexpected error while compiling internal selector: {}", e))?;

        let mut dict = prepare_methods(&self.engine.privates(), false)
            .map_err(|(i, s)| format_err!("{}", s.replace("_name_", &self.engine.global_name(i).unwrap())))?;

        insert_methods(&mut dict.0, &mut dict.1, &self.engine.internals(), false)
            .map_err(|(i, s)| format_err!("{}", s.replace("_name_", &self.engine.internal_name(i).unwrap())))?;

        insert_methods(&mut dict.0, &mut dict.1, &self.publics_filtered(remove_ctor), false)
            .map_err(|(i, s)| format_err!("{}", s.replace("_name_", &self.engine.global_name(i).unwrap())))?;

        let mut entry_points = vec![];
        for id in -2..1i32 {
            let key = SliceData::load_cell(id.serialize()?)?;
            let value = dict.0.remove(key)?;
            entry_points.push(value.unwrap_or_default());
        }

        internal_selector.0.append_reference(SliceData::load_cell(dict.0.data().cloned().unwrap_or_default())?);
        self.dbgmap.append(&mut dict.1);

        let version = self.engine.version();
        if let Some(version) = version {
            let version = version.as_bytes();
            internal_selector.0.append_reference(SliceData::from_raw(version.to_vec(), version.len() * 8));
        }

        // adjust hash of internal_selector cell
        let hash = internal_selector.0.cell().repr_hash();
        assert_eq!(internal_selector.1.len(), 1);
        let entry = internal_selector.1.first_entry().unwrap();
        self.dbgmap.insert(hash, entry.clone());

        let entry_selector_text = vec![
            Line::new("PUSHREFCONT\n", "<entry-selector>", 1),
            Line::new("POPCTR c3\n",   "<entry-selector>", 2),
            Line::new("DUP\n",         "<entry-selector>", 3),
            Line::new("IFNOTJMPREF\n", "<entry-selector>", 4),  //  0 - internal transaction
            Line::new("DUP\n",         "<entry-selector>", 5),
            Line::new("EQINT -1\n",    "<entry-selector>", 6),
            Line::new("IFJMPREF\n",    "<entry-selector>", 7),  // -1 - external transaction
            Line::new("DUP\n",         "<entry-selector>", 8),
            Line::new("EQINT -2\n",    "<entry-selector>", 9),
            Line::new("IFJMPREF\n",    "<entry-selector>", 10), // -2 - ticktock transaction
            Line::new("THROW 11\n",    "<entry-selector>", 11),
        ];

        let mut entry_selector = compile_code_debuggable(entry_selector_text)
            .map_err(|e| format_err!("compilation failed: {}", e))?;

        entry_selector.0.append_reference(internal_selector.0);
        entry_points.reverse();
        for entry in entry_points {
            entry_selector.0.append_reference(entry);
        }

        // adjust hash of entry_selector cell
        let hash = entry_selector.0.cell().repr_hash();
        assert_eq!(entry_selector.1.len(), 1);
        let entry = entry_selector.1.first_entry().unwrap();
        self.dbgmap.insert(hash, entry.clone());

        let func_id = match self.engine.func_upgrade() {
            SelectorVariant::Default => {
                return Ok(entry_selector.0.cell().clone());
            }
            SelectorVariant::UpdateFunc => {
                1666
            }
            SelectorVariant::UpdateOldSol => {
                2
            }
        };
        let func_upgrade_text = vec![
            Line::new(format!("PUSHINT {}\n", func_id).as_str(),    "<func-upgrade-code>", 1),
            Line::new("EQUAL\n",           "<func-upgrade-code>", 2),
            Line::new("THROWIFNOT 79\n",   "<func-upgrade-code>", 3),

            Line::new("PUSHREF\n",         "<func-upgrade-code>", 4),
            Line::new("DUP\n",             "<func-upgrade-code>", 5),
            Line::new("SETCODE\n",         "<func-upgrade-code>", 6),
            Line::new("CTOS\n",            "<func-upgrade-code>", 7),
            Line::new("PLDREF\n",          "<func-upgrade-code>", 8),
            Line::new("CTOS\n",            "<func-upgrade-code>", 9),
            Line::new("BLESS\n",           "<func-upgrade-code>", 10),
            Line::new("POP C3\n",          "<func-upgrade-code>", 12),
            Line::new("CALL 2\n",          "<func-upgrade-code>", 13),
        ];
        let mut func_upgrade_code = compile_code_debuggable(func_upgrade_text)
            .map_err(|e| format_err!("compilation failed: {}", e))?;
        assert_eq!(func_upgrade_code.1.len(), 1);
        let old_hash = func_upgrade_code.0.cell().repr_hash();
        let entry = func_upgrade_code.1.get(&old_hash).unwrap();
        func_upgrade_code.0.append_reference(entry_selector.0);

        let hash = func_upgrade_code.0.cell().repr_hash();
        self.dbgmap.insert(hash, entry.clone());

        Ok(func_upgrade_code.0.cell().clone())
    }

    pub fn debug_print(&self) {
        self.engine.debug_print();
    }
}

pub fn load_from_file(contract_file: &str) -> Result<StateInit> {
    let mut csor = Cursor::new(std::fs::read(contract_file)?);
    let mut cell = deserialize_cells_tree(&mut csor)?.remove(0);
    // try appending a dummy library cell if there is no such cell in the tvc file
    if cell.references_count() == 2 {
        let mut adjusted_cell = BuilderData::from(cell);
        adjusted_cell.checked_append_reference(Cell::default())?;
        cell = adjusted_cell.into_cell()?;
    }
    StateInit::construct_from_cell(cell)
}

pub fn load_stateinit(file_name: &str) -> Result<(SliceData, Vec<u8>)> {
    let mut orig_bytes = Vec::new();
    let mut f = File::open(file_name)?;
    f.read_to_end(&mut orig_bytes)?;

    let mut cur = Cursor::new(orig_bytes.clone());
    let (root_cells, _mode, _x, _y) = deserialize_cells_tree_ex(&mut cur)?;
    let mut root = root_cells[0].clone();
    if root.references_count() == 2 { // append empty library cell
        let mut adjusted_cell = BuilderData::from(root);
        adjusted_cell.checked_append_reference(Cell::default())?;
        root = adjusted_cell.into_cell()?;
    }
    Ok((SliceData::load_cell(root)?, orig_bytes))
}

pub fn get_now() -> u32 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32
}

#[cfg(test)]
mod tests {
    use crate::abi;
    use crate::printer::get_version_mycode_aware;
    use crate::testcall::{TraceLevel, load_debug_info, load_config, call_contract, TestCallParams, MsgInfo};
    use super::*;

    use std::{fs::File, str::FromStr};
    use std::path::Path;

    fn compile_into_trash(prog: &mut Program, base: &str) -> Result<String> {
        let prefix = format!("./trash/{}", base);
        prog.compile_to_file_ex(&prefix, None, true, false)
    }

    fn call_contract_1<F>(
        smc_file: &str,
        address: &str,
        smc_balance: Option<&str>,
        msg_info: MsgInfo,
        config_file: Option<&str>,
        key_file: Option<Option<&str>>,
        ticktock: Option<i8>,
        gas_limit: Option<i64>,
        action_decoder: Option<F>,
        trace_level: TraceLevel,
        debug_map_filename: &str
    ) -> Result<i32>
        where F: Fn(SliceData, bool)
    {
        let wc = match msg_info.balance {
            Some(_) => 0,
            None => if ticktock.is_some() { -1 } else { 0 },
        };

        let addr = if address.find(':').is_none() {
            format!("{}:{}", wc, address)
        } else {
            address.to_owned()
        };
        let addr = MsgAddressInt::from_str(&addr)?;

        let state_init = load_from_file(smc_file)?;
        let debug_info = load_debug_info(debug_map_filename);
        let config_cell = config_file.and_then(load_config);
        let (exit_code, state_init, is_vm_success) = call_contract(
            addr,
            state_init,
            TestCallParams {
                balance: smc_balance,
                msg_info,
                config: config_cell,
                key_file,
                ticktock,
                gas_limit,
                action_decoder,
                trace_level,
                debug_info,
                capabilities: 0x42E, // default
            }
        )?;

        if is_vm_success {
            // save_to_file(state_init, Some(smc_file), 0, false)?;
            // TODO: save to file
            println!("Contract persistent data updated");
        }

        Ok(exit_code)
    }

    fn call_contract_2<F>(
        contract_file: &str,
        body: Option<SliceData>,
        key_file: Option<Option<&str>>,
        trace_level: TraceLevel,
        decode_c5: bool,
        msg_balance: Option<&str>,
        ticktock: Option<i8>,
        src: Option<&str>,
        balance: Option<&str>,
        now: u32,
        action_decoder: F,
    ) -> i32
        where F: Fn(SliceData, bool)
    {
        // let file = format!("{}.tvc", contract_file);
        call_contract_1(
            &contract_file,
            "0000000000000000000000000000000000000000000000000000000000000000",
            balance,
            MsgInfo{
                balance: msg_balance,
                src,
                now,
                bounced: false,
                body
            },
            None,
            key_file,
            ticktock,
            None,
            if decode_c5 { Some(action_decoder) } else { None },
            trace_level,
            ""
        ).unwrap()
    }

    #[test]
    fn test_ticktock() {
        let sources = vec![Path::new("./tests/test_stdlib_sol.tvm"),
                                     Path::new("./tests/ticktock.code")];
        let parser = ParseEngine::new(sources, None);
        assert_eq!(parser.is_ok(), true);
        let mut prog = Program::new(parser.unwrap());

        let name = compile_into_trash(&mut prog, "test_ticktock").unwrap();
        assert_eq!(call_contract_2(name.as_str(), None, None, TraceLevel::None, false, None, Some(-1), None, None, 0, |_b,_i| {}), 0);
    }

    #[test]
    fn test_call_with_gas_limit() {
        let sources = vec![
            Path::new("./tests/test_stdlib_sol.tvm"),
            Path::new("./tests/Wallet.code")
        ];

        let abi = abi::load_abi_json_string("./tests/Wallet.abi.json").unwrap();

        let parser = ParseEngine::new(sources, Some(abi));
        assert_eq!(parser.is_ok(), true);

        let mut prog = Program::new(parser.unwrap());

        let name = compile_into_trash(&mut prog, "test_call_with_gas_limit").unwrap();
        let body = abi::build_abi_body("./tests/Wallet.abi.json", "constructor", "{}", None, None, false, None)
            .unwrap();

        let exit_code = call_contract_1(
            name.as_str(),
            "0000000000000000000000000000000000000000000000000000000000000000",
            Some("10000000000"), //account balance 10T
            MsgInfo {
                balance: Some("1000000000"), // msg balance = 1T
                src: None,
                now: 1,
                bounced: false,
                body: Some(SliceData::load_builder(body).unwrap())
            },
            None,
            None,
            None,
            Some(3000), // gas limit
            Some(|_, _| {}),
            TraceLevel::None,
            ""
        );
        // must equal to out of gas exception
        assert!(exit_code.is_ok());
        assert_eq!(exit_code.unwrap(), 13);
    }

    #[test]
    fn test_debug_map() {
        // suppress interference from test_call_with_gas_limit
        std::fs::copy("tests/Wallet.code", "tests/Wallet2.code").unwrap();

        let sources = vec![Path::new("tests/test_stdlib_sol.tvm"),
                                     Path::new("tests/Wallet2.code")];
        let abi = abi::load_abi_json_string("tests/Wallet.abi.json").unwrap();

        let parser = ParseEngine::new(sources, Some(abi));
        assert_eq!(parser.is_ok(), true);
        let mut prog = Program::new(parser.unwrap());

        let contract_file = compile_into_trash(&mut prog, "test_debug_map").unwrap();

        let debug_map_filename = String::from("tests/Wallet2.map.json");
        let debug_map_file = File::create(&debug_map_filename).unwrap();
        serde_json::to_writer_pretty(debug_map_file, &prog.dbgmap).unwrap();

        let name = contract_file.split('.').next().unwrap();
        let body = abi::build_abi_body("tests/Wallet.abi.json", "constructor", "{}", None, None, false, None)
            .unwrap();

        let exit_code = call_contract_1(
            &contract_file,
            &name,
            Some("10000000000"), //account balance 10T
            MsgInfo {
                balance: Some("1000000000"), // msg balance = 1T
                src: None,
                now: 1,
                bounced: false,
                body: Some(SliceData::load_builder(body).unwrap())
            },
            None,
            None,
            None,
            None,
            Some(|_, _| {}),
            TraceLevel::Full,
            &debug_map_filename
        );
        assert!(exit_code.is_ok());
        assert_eq!(exit_code.unwrap(), 0);
    }

    fn get_version(filename: &str) -> Result<String> {
        let parser = ParseEngine::new(vec![Path::new(filename)], None);
        assert_eq!(parser.is_ok(), true);
        let mut prog = Program::new(parser.unwrap());

        let file_name = compile_into_trash(&mut prog, "get_version").unwrap();

        let dec = scheme::TVC::construct_from_file(&file_name).unwrap();
        let tvc = if let scheme::TvmSmc::TvcFrst(t) = dec.tvc {
            t
        } else {
            panic!("tvc must be not none in tests")
        };

        get_version_mycode_aware(Some(&tvc.code))
    }

    #[test]
    fn test_get_version() {
        assert_eq!(
            "0.43.0+commit.e8c3d877.mod.Linux.g++".to_string(),
            get_version("tests/get-version1.code").unwrap_or("".to_string())
        );

        assert_eq!(
            "0.43.0+commit.e8c3d877.mod.Linux.g++".to_string(),
            get_version("tests/get-version2.code").unwrap_or("".to_string())
        );

        assert_eq!(
            "not found (cell underflow)".to_string(),
            get_version("tests/get-version3.code").unwrap_err().to_string()
        );
    }

    #[test]
    fn test_mycode() {
        let sources = vec![Path::new("tests/test_stdlib_sol.tvm"), Path::new("tests/mycode.code")];
        let abi = abi::load_abi_json_string("tests/mycode.abi.json").unwrap();

        let parser = ParseEngine::new(sources, Some(abi));
        assert_eq!(parser.is_ok(), true);
        let mut prog = Program::new(parser.unwrap());

        let contract_file = compile_into_trash(&mut prog, "test_mycode").unwrap();

        let name = contract_file.split('.').next().unwrap();
        let body = abi::build_abi_body("tests/mycode.abi.json", "constructor", "{}", None, None, false, None)
            .unwrap();
        let exit_code = call_contract_1(
            &contract_file,
            &name,
            Some("10000000000"), //account balance 10T
            MsgInfo {
                balance: Some("1000000000"), // msg balance = 1T
                src: None,
                now: 1,
                bounced: false,
                body: Some(SliceData::load_builder(body).unwrap())
            },
            None,
            None,
            None,
            None,
            Some(|_, _| {}),
            TraceLevel::None,
            ""
        );
        assert!(exit_code.is_ok());
        assert_eq!(exit_code.unwrap(), 0);
    }
}
