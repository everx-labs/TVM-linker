/*
 * Copyright 2018-2019 TON DEV SOLUTIONS LTD.
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
use base64::encode;
use crc16::*;
use ed25519_dalek::*;
use std::io::Cursor;
use std::io::Write;
use std::collections::HashMap;
use std::time::SystemTime;
use methdict::*;
use ton_block::*;
use ton_labs_assembler::{Line, Lines, compile_code_debuggable, DbgInfo};
use ton_types::cells_serialization::{BagOfCells, deserialize_cells_tree};
use ton_types::{Cell, SliceData, BuilderData, IBitstring};
use ton_types::dictionary::{HashmapE, HashmapType};
use parser::{ptr_to_builder, ParseEngine, ParseEngineResults};
use testcall::TraceLevel;

pub struct Program {
    language: Option<String>,
    engine: ParseEngineResults,
    keypair: Option<Keypair>,
    pub dbgmap: DbgInfo,
}

impl Program {
    pub fn new(parser: ParseEngine) -> Self {
        Program {
            language: None,
            engine: ParseEngineResults::new(parser),
            keypair: None,
            dbgmap: DbgInfo::new(),
        }
    }

    pub fn set_keypair(&mut self, pair: Keypair) {
        self.keypair = Some(pair);
    }

    pub fn set_language(&mut self, lang: Option<&str>) {
        self.language = lang.map(|s| s.to_owned());
    }

    pub fn data(&self) -> std::result::Result<Cell, String> {
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
        let key = ptr_to_builder(persistent_base)?.into();
        BuilderData::with_raw(bytes.to_vec(), PUBLIC_KEY_LENGTH * 8)
            .and_then(|data| data_dict.set(key, &data.into()))
            .map_err(|e| format!("failed to pack pubkey to data dictionary: {}", e))?;
        let mut builder = BuilderData::new();
        builder
            .append_bit_one().unwrap()
            .checked_append_reference(data_dict.data().unwrap().clone()).unwrap();
        Ok(builder.into())
    }

    #[allow(dead_code)]
    pub fn entry(&self) -> Lines {
        self.engine.entry()
    }

    pub fn internal_method_dict(&mut self) -> std::result::Result<Option<Cell>, String> {
        let dict = prepare_methods(&self.engine.privates(), true)
            .map_err(|e| e.1.replace("_name_", &self.engine.global_name(e.0).unwrap()))?;
        self.dbgmap.map.append(&mut dict.1.map.clone());
        Ok(dict.0.data().map(|cell| cell.clone()))
    }

    fn publics_filtered(&self, remove_ctor: bool) -> HashMap<u32, Lines> {
        self.engine.publics().into_iter()
            .filter(|(k, _)| 
                !(remove_ctor && self.engine.global_name(*k).unwrap_or_default() == "constructor")
            ).collect()
    }

    pub fn public_method_dict(&mut self, remove_ctor: bool) -> std::result::Result<Option<Cell>, String> {
        let mut dict = prepare_methods(&self.engine.internals(), true)
            .map_err(|e| e.1.replace("_name_", &self.engine.internal_name(e.0).unwrap()) )?;

        insert_methods(&mut dict.0, &mut dict.1, &self.publics_filtered(remove_ctor), true)
            .map_err(|e| e.1.replace("_name_", &self.engine.global_name(e.0).unwrap()) )?;

        self.dbgmap.map.append(&mut dict.1.map);

        Ok(dict.0.data().map(|cell| cell.clone()))
    }

    #[allow(dead_code)]
    pub fn compile_to_file(&mut self, wc: i8) -> std::result::Result<String, String> {
        self.compile_to_file_ex(wc, None, None, None, false, None)
    }

    pub fn compile_to_file_ex(
        &mut self,
        wc: i8,
        abi_file: Option<&str>,
        ctor_params: Option<&str>,
        out_file: Option<&str>,
        trace: bool,
        data_filename: Option<&str>,
    ) -> std::result::Result<String, String> {
        let mut state_init = self.compile_to_state()?;
        if let Some(ctor_params) = ctor_params {
            state_init = self.apply_constructor(state_init, abi_file.unwrap(), ctor_params, trace)?;
        }
        if let Some(data_filename) = data_filename {
            let mut data_cursor = Cursor::new(std::fs::read(data_filename).unwrap());
            let data_cell = deserialize_cells_tree(&mut data_cursor).unwrap().remove(0);
            state_init.set_data(data_cell);
        }
        save_to_file(state_init, out_file, wc)
    }

    fn apply_constructor(
        &mut self,
        state_init: StateInit,
        abi_file: &str,
        ctor_params : &str,
        trace: bool
    ) -> std::result::Result<StateInit, String> {
        use ton_types::{AccountId, UInt256};
        use ton_vm::stack::integer::IntegerData;
        use testcall::{call_contract_ex, MsgInfo};
        use abi;

        let action_decoder = |_b,_i| {};

        let body: SliceData = abi::build_abi_body(
            abi_file,
            "constructor",
            ctor_params,
            None,   // header,
            None,   // key_file,
            false   // is_internal
        )?.into();

        let (exit_code, mut state_init) = call_contract_ex(
            AccountId::from(UInt256::default()),
            IntegerData::zero(),
            state_init,
            None, // debug_info
            None, // balance,
            MsgInfo{
                balance: None,
                src: None,
                now: get_now(),
                bounced: false,
                body: Some(body),
            },
            None, // config
            None, // key_file,
            None, // ticktock,
            None, // gas_limit,
            Some(action_decoder),
            if trace { TraceLevel::Full } else { TraceLevel::None }
        );

        if exit_code == 0 || exit_code == 1 {
            // TODO: check that no action is fired.
            // Rebuild code with removed constructor
            state_init.set_code(self.compile_asm(true)?);
            Ok(state_init)
        } else {
            Err(format!("Constructor failed ec = {}", exit_code))
        }
    }

    fn compile_to_state(&mut self) -> std::result::Result<StateInit, String> {
        let mut state = StateInit::default();
        state.set_code(self.compile_asm(false)?);
        state.set_data(self.data()?);
        Ok(state)
    }

    fn compile_asm_old(&mut self, remove_ctor: bool) -> std::result::Result<Cell, String> {
        let internal_selector_text = vec![
            Line::new("DICTPUSHCONST 32\n", "<internal-selector>", 1),
            Line::new("DICTUGETJMP\n",      "<internal-selector>", 2),
        ];
        let mut internal_selector = compile_code_debuggable(internal_selector_text)
            .map_err(|_| "unexpected TVM error while compiling internal selector".to_string())?;
        internal_selector.0.append_reference(self.internal_method_dict()?.unwrap_or_default().into());

        // adjust hash of internal_selector cell
        let hash = internal_selector.0.cell().repr_hash().to_hex_string();
        assert!(internal_selector.1.map.len() == 1);
        let entry = internal_selector.1.map.iter().next().unwrap();
        self.dbgmap.map.insert(hash, entry.1.clone());

        let mut main_selector = compile_code_debuggable(self.entry())
            .map_err(|e| e.to_string())?;
        main_selector.0.append_reference(self.public_method_dict(remove_ctor)?.unwrap_or_default().into());
        main_selector.0.append_reference(internal_selector.0);

        // adjust hash of main_selector cell
        let hash = main_selector.0.cell().repr_hash().to_hex_string();
        assert!(main_selector.1.map.len() == 1);
        let entry = main_selector.1.map.iter().next().unwrap();
        self.dbgmap.map.insert(hash, entry.1.clone());

        Ok(main_selector.0.cell().clone())
    }

    fn compile_asm(&mut self, remove_ctor: bool) -> std::result::Result<Cell, String> {
        if !self.entry().is_empty() {
            // TODO wipe out the old behavior
            return self.compile_asm_old(remove_ctor);
        }

        let internal_selector_text = vec![
            // indirect jump
            Line::new("DICTPUSHCONST 32\n", "<internal-selector>", 1),
            Line::new("DICTUGETJMP\n",      "<internal-selector>", 2),
        ];

        let mut internal_selector = compile_code_debuggable(internal_selector_text)
            .map_err(|_| "unexpected TVM error while compiling internal selector".to_string())?;

        let mut dict = prepare_methods(&self.engine.privates(), false)
            .map_err(|e| e.1.replace("_name_", &self.engine.global_name(e.0).unwrap()))?;

        insert_methods(&mut dict.0, &mut dict.1, &self.engine.internals(), false)
            .map_err(|e| e.1.replace("_name_", &self.engine.internal_name(e.0).unwrap()) )?;

        insert_methods(&mut dict.0, &mut dict.1, &self.publics_filtered(remove_ctor), false)
            .map_err(|e| e.1.replace("_name_", &self.engine.global_name(e.0).unwrap()) )?;

        let mut entry_points = vec![];
        for id in -2..1 {
            let key = id.write_to_new_cell().unwrap().into();
            let value = dict.0.remove(key).unwrap();
            entry_points.push(value.unwrap_or(SliceData::default()));
        }

        internal_selector.0.append_reference(SliceData::from(dict.0.data().unwrap_or(&Cell::default())));
        self.dbgmap.map.append(&mut dict.1.map.clone());

        let version = self.engine.version();
        match version {
            Some(version) => {
                let version = version.as_bytes();
                internal_selector.0.append_reference(SliceData::from_raw(version.to_vec(), version.len() * 8));
            },
            None => {}
        }

        // adjust hash of internal_selector cell
        let hash = internal_selector.0.cell().repr_hash().to_hex_string();
        assert_eq!(internal_selector.1.map.len(), 1);
        let entry = internal_selector.1.map.iter().next().unwrap();
        self.dbgmap.map.insert(hash, entry.1.clone());

        let entry_selector_text = vec![
            Line::new("SETCP 0\n",     "<entry-selector>", 1),
            Line::new("PUSHREFCONT\n", "<entry-selector>", 2),
            Line::new("POPCTR c3\n",   "<entry-selector>", 3),
            Line::new("DUP\n",         "<entry-selector>", 4),
            Line::new("IFNOTJMPREF\n", "<entry-selector>", 5),  //  0 - internal transaction
            Line::new("DUP\n",         "<entry-selector>", 6),
            Line::new("EQINT -1\n",    "<entry-selector>", 7),
            Line::new("IFJMPREF\n",    "<entry-selector>", 8),  // -1 - external transaction
            Line::new("DUP\n",         "<entry-selector>", 9),
            Line::new("EQINT -2\n",    "<entry-selector>", 10),
            Line::new("IFJMPREF\n",    "<entry-selector>", 11), // -2 - ticktock transaction
            Line::new("THROW 11\n",    "<entry-selector>", 12),
        ];

        let mut entry_selector = compile_code_debuggable(entry_selector_text.clone())
            .map_err(|e| e.to_string())?;

        entry_selector.0.append_reference(internal_selector.0);
        entry_points.reverse();
        for entry in entry_points {
            entry_selector.0.append_reference(entry);
        }

        // adjust hash of entry_selector cell
        let hash = entry_selector.0.cell().repr_hash().to_hex_string();
        assert_eq!(entry_selector.1.map.len(), 1);
        let entry = entry_selector.1.map.iter().next().unwrap();
        self.dbgmap.map.insert(hash, entry.1.clone());

        if !self.engine.save_my_code() {
            return Ok(entry_selector.0.cell().clone())
        }

        let save_my_code_text = vec![
            Line::new("PUSHREF {\n",                      "<save-my-code>", 1),
            Line::new("  DUP\n",                          "<save-my-code>", 1),
            Line::new("  XCHG s0, s2\n",                  "<save-my-code>", 1),
            Line::new("  NEWC\n",                         "<save-my-code>", 1),
            Line::new("  STSLICECONST x8820d068a57f\n",   "<save-my-code>", 1),
            Line::new("  STSLICECONST xed1fdb35\n",       "<save-my-code>", 1),
            Line::new("  STREF\n",                        "<save-my-code>", 1),
            Line::new("  STSLICE\n",                      "<save-my-code>", 1),
            Line::new("  PUSH c7\n",                      "<save-my-code>", 1),
            Line::new("  FIRST\n",                        "<save-my-code>", 1),
            Line::new("  UNTUPLE 10\n",                   "<save-my-code>", 1),
            Line::new("  DROP\n",                         "<save-my-code>", 1),
            Line::new("  PUSH s9\n",                      "<save-my-code>", 1),
            Line::new("  NULL\n",                         "<save-my-code>", 1),
            Line::new("  TUPLE 11\n",                     "<save-my-code>", 1),
            Line::new("  SINGLE\n",                       "<save-my-code>", 1),
            Line::new("  POP c7\n",                       "<save-my-code>", 1),
            Line::new("  DROP\n",                         "<save-my-code>", 1),
            Line::new("  DEPTH\n",                        "<save-my-code>", 1),
            Line::new("  DEC\n",                          "<save-my-code>", 1),
            Line::new("  PUSHINT -1\n",                   "<save-my-code>", 1),
            Line::new("  BLESSVARARGS\n",                 "<save-my-code>", 1),
            Line::new("  JMPX\n",                         "<save-my-code>", 1),
            Line::new("}\n",                              "<save-my-code>", 1),
            Line::new("DUP\n",                            "<save-my-code>", 1),
            Line::new("CTOS\n",                           "<save-my-code>", 1),
            Line::new("DEPTH\n",                          "<save-my-code>", 1),
            Line::new("DEC\n",                            "<save-my-code>", 1),
            Line::new("PUSHINT -1\n",                     "<save-my-code>", 1),
            Line::new("BLESSVARARGS\n",                   "<save-my-code>", 1),
            Line::new("JMPXDATA\n",                       "<save-my-code>", 1),
            Line::new("JMPREF\n",                         "<save-my-code>", 1),
        ];
        let mut save_my_code = compile_code_debuggable(save_my_code_text.clone())
            .map_err(|e| e.to_string())?;
        save_my_code.0.append_reference(entry_selector.0);

        // TODO adjust debug map

        Ok(save_my_code.0.cell().clone())
    }

    pub fn debug_print(&self) {
        self.engine.debug_print();
    }
}

pub fn save_to_file(state: StateInit, name: Option<&str>, wc: i8) -> std::result::Result<String, String> {
    let root_cell = state.write_to_new_cell()
        .map_err(|e| format!("Serialization failed: {}", e))?
        .into();
    let mut buffer = vec![];
    BagOfCells::with_root(&root_cell).write_to(&mut buffer, false)
        .map_err(|e| format!("BOC failed: {}", e))?;

    let mut print_filename = false;
    let address = state.hash().unwrap();
    let file_name = if name.is_some() {
        format!("{}", name.unwrap())
    } else {
        print_filename = true;
        format!("{:x}.tvc", address)
    };

    let mut file = std::fs::File::create(&file_name).unwrap();
    file.write_all(&buffer).map_err(|e| format!("Write to file failed: {}", e))?;

    if print_filename {
        println! ("Saved contract to file {}", &file_name);
        println!("testnet:");
        println!("Non-bounceable address (for init): {}", &calc_userfriendly_address(wc, address.as_slice(), false, true));
        println!("Bounceable address (for later access): {}", &calc_userfriendly_address(wc, address.as_slice(), true, true));
        println!("mainnet:");
        println!("Non-bounceable address (for init): {}", &calc_userfriendly_address(wc, address.as_slice(), false, false));
        println!("Bounceable address (for later access): {}", &calc_userfriendly_address(wc, address.as_slice(), true, false));
    }
    Ok(file_name)
}

fn calc_userfriendly_address(wc: i8, addr: &[u8], bounce: bool, testnet: bool) -> String {
    let mut bytes: Vec<u8> = vec![];
    bytes.push(if bounce { 0x11 } else { 0x51 } + if testnet { 0x80 } else { 0 });
    bytes.push(wc as u8);
    bytes.extend_from_slice(addr);
    let crc = State::<XMODEM>::calculate(&bytes);
    bytes.extend_from_slice(&crc.to_be_bytes());
    encode(&bytes)
}

pub fn load_from_file(contract_file: &str) -> StateInit {
    let mut csor = Cursor::new(std::fs::read(contract_file).unwrap());
    let mut cell = deserialize_cells_tree(&mut csor).unwrap().remove(0);
    // try appending a dummy library cell if there is no such cell in the tvc file
    if cell.references_count() == 2 {
        let mut adjusted_cell = BuilderData::from(cell);
        adjusted_cell.append_reference(BuilderData::default());
        cell = adjusted_cell.into();
    }
    StateInit::construct_from_cell(cell).expect("StateInit construction failed")
}

pub fn get_now() -> u32 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32
}

// Debug function to dump structured cell content with hashes
#[allow(dead_code)]
fn dump_cell(cell: &Cell, pfx: &str) {
    println!("{}# {:?}", pfx, cell.repr_hash());
    println!("{}{}", pfx, cell.to_hex_string(false));
    for i in 0..cell.references_count() {
        let child = cell.reference(i).unwrap();
        dump_cell(&child, &(pfx.to_owned() + "  "));
    }
}

#[cfg(test)]
mod tests {
    use abi;
    use std::fs::File;
    use std::path::Path;
    use super::*;
    use testcall::{perform_contract_call, call_contract, MsgInfo};

    #[ignore] // due to offline constructor
    #[test]
    fn test_comm_var_addresses() {
        let sources = vec![Path::new("./tests/test_stdlib.tvm"),
                                     Path::new("./tests/comm_test2.s")];
        let parser = ParseEngine::new(sources, None);
        assert_eq!(parser.is_ok(), true);
        let mut prog = Program::new(parser.unwrap());
        let body = {
            let mut b = BuilderData::new();
            b.append_u32(abi::gen_abi_id(None, "main")).unwrap();
            b.append_reference(BuilderData::new());
            Some(b.into())
        };
        let contract_file = prog.compile_to_file(0).unwrap();
        let name = contract_file.split('.').next().unwrap();
        assert_eq!(perform_contract_call(name, body, Some(None), TraceLevel::None, false, None, None, None, None, 0, |_b,_i| {}), 0);
    }

    #[ignore] // due to offline constructor
    #[test]
    fn test_asciz_var() {
        let sources = vec![Path::new("./tests/test_stdlib.tvm"),
                                     Path::new("./tests/asci_test1.s")];
        let parser = ParseEngine::new(sources, None);
        assert_eq!(parser.is_ok(), true);
        let mut prog = Program::new(parser.unwrap());
        let body = {
            let mut b = BuilderData::new();
            b.append_u32(abi::gen_abi_id(None, "main")).unwrap();
            Some(b.into())
        };
        let contract_file = prog.compile_to_file(0).unwrap();
        let name = contract_file.split('.').next().unwrap();
        assert_eq!(perform_contract_call(name, body, Some(None), TraceLevel::None, false, None, None, None, None, 0, |_b,_i| {}), 0);
    }

    #[test]
    #[ignore]
    //TODO: use when stdlib will be modified to store sender key.
    fn test_sender_pubkey() {
        let sources = vec![Path::new("./tests/test_stdlib_c.tvm"),
                                     Path::new("./tests/sign-test.s")];
        let parser = ParseEngine::new(sources, None);
        assert_eq!(parser.is_ok(), true);
        let mut prog = Program::new(parser.unwrap());
        let body = {
            let buf = hex::decode("000D6E4079").unwrap();
            let buf_bits = buf.len() * 8;
            Some(BuilderData::with_raw(buf, buf_bits).unwrap().into())
        };
        let contract_file = prog.compile_to_file(0).unwrap();
        let name = contract_file.split('.').next().unwrap();

        assert_eq!(perform_contract_call(name, body, Some(Some("key1")), TraceLevel::None, false, None, None, None, None, 0, |_b,_i| {}), 0);
    }

    #[test]
    fn test_bouncable_address() {
        let addr = hex::decode("fcb91a3a3816d0f7b8c2c76108b8a9bc5a6b7a55bd79f8ab101c52db29232260").unwrap();
        let addr = calc_userfriendly_address(-1, &addr, true, true);
        assert_eq!(addr, "kf/8uRo6OBbQ97jCx2EIuKm8Wmt6Vb15+KsQHFLbKSMiYIny");
    }

    #[test]
    fn test_ticktock() {
        let sources = vec![Path::new("./tests/test_stdlib_sol.tvm"),
                                     Path::new("./tests/ticktock.code")];
        let parser = ParseEngine::new(sources, None);
        assert_eq!(parser.is_ok(), true);
        let mut prog = Program::new(parser.unwrap());
        let contract_file = prog.compile_to_file(-1).unwrap();
        let name = contract_file.split('.').next().unwrap();

        assert_eq!(perform_contract_call(name, None, None, TraceLevel::None, false, None, Some(-1), None, None, 0, |_b,_i| {}), 0);
    }

    #[ignore] // due to offline constructor
    #[test]
    fn test_recursive_call() {
        let sources = vec![Path::new("./tests/test_stdlib.tvm"),
                                     Path::new("./tests/test_recursive.code")];
        let parser = ParseEngine::new(sources, None);
        assert_eq!(parser.is_ok(), true);
        let mut prog = Program::new(parser.unwrap());
        let contract_file = prog.compile_to_file(-1).unwrap();
        let name = contract_file.split('.').next().unwrap();
        let body = {
            let mut b = BuilderData::new();
            b.append_u32(abi::gen_abi_id(None, "main")).unwrap();
            Some(b.into())
        };

        assert_eq!(perform_contract_call(name, body, Some(Some("key1")), TraceLevel::None, false, None, None, None, None, 0, |_b,_i| {}), 0);
    }

    #[ignore] // due to offline constructor
    #[test]
    fn test_public_and_private() {
        let sources = vec![Path::new("./tests/test_stdlib.tvm"),
                                     Path::new("./tests/test_public.code")];

        let abi_str = abi::load_abi_json_string("./tests/test_public.abi.json").unwrap();
        let abi = abi::load_abi_contract(&abi_str).unwrap();

        let parser = ParseEngine::new(sources, Some(abi_str));
        assert_eq!(parser.is_ok(), true);
        let mut prog = Program::new(parser.unwrap());

        let contract_file = prog.compile_to_file(0).unwrap();
        let name = contract_file.split('.').next().unwrap();
        let body1 = {
            let mut b = BuilderData::new();
            b.append_u32(abi::gen_abi_id(None, "sum")).unwrap();
            b.append_reference(BuilderData::new());
            Some(b.into())
        };

        assert_eq!(perform_contract_call(name, body1, None, TraceLevel::None, false, None, None, None, None, 0, |_b,_i| {}), 0);

        let body2 = {
            let mut b = BuilderData::new();
            b.append_u32(abi::gen_abi_id(None, "sum_p")).unwrap();
            b.append_reference(BuilderData::new());
            Some(b.into())
        };
        assert!(perform_contract_call(name, body2, None, TraceLevel::None, false, None, None, None, None, 0, |_b,_i| {}) != 0);

        let body3 = {
            let mut b = BuilderData::new();
            b.append_u32(abi::gen_abi_id(Some(abi), "sum2")).unwrap();
            b.append_reference(BuilderData::new());
            Some(b.into())
        };
        assert_eq!(perform_contract_call(name, body3, None, TraceLevel::None, false, None, None, None, None, 0, |_b,_i| {}), 0);
    }

    #[test]
    fn test_call_with_gas_limit() {
        let sources = vec![Path::new("./tests/test_stdlib_sol.tvm"),
                                     Path::new("./tests/Wallet.code")];
        let abi = abi::load_abi_json_string("./tests/Wallet.abi.json").unwrap();

        let parser = ParseEngine::new(sources, Some(abi));
        assert_eq!(parser.is_ok(), true);
        let mut prog = Program::new(parser.unwrap());

        let contract_file = prog.compile_to_file(0).unwrap();
        let name = contract_file.split('.').next().unwrap();
        let body = abi::build_abi_body("./tests/Wallet.abi.json", "constructor", "{}", None, None, false)
            .unwrap();
        let exit_code = call_contract(
            &name,
            Some("10000000000"), //account balance 10T
            MsgInfo {
                balance: Some("1000000000"), // msg balance = 1T
                src: None,
                now: 1,
                bounced: false,
                body: Some(body.into())
            },
            None,
            None,
            None,
            Some(3000), // gas limit
            Some(|_, _| {}),
            TraceLevel::None,
            String::from(""),
        );
        // must equal to out of gas exception
        assert_eq!(exit_code, 13);
    }

    #[test]
    fn test_debug_map() {
        let sources = vec![Path::new("tests/test_stdlib_sol.tvm"),
                                     Path::new("tests/Wallet.code")];
        let abi = abi::load_abi_json_string("tests/Wallet.abi.json").unwrap();

        let parser = ParseEngine::new(sources, Some(abi));
        assert_eq!(parser.is_ok(), true);
        let mut prog = Program::new(parser.unwrap());

        let contract_file = prog.compile_to_file(0).unwrap();
        let debug_map_filename = String::from("tests/Wallet.map.json");
        let debug_map_file = File::create(&debug_map_filename).unwrap();
        serde_json::to_writer_pretty(debug_map_file, &prog.dbgmap).unwrap();

        let name = contract_file.split('.').next().unwrap();
        let body = abi::build_abi_body("tests/Wallet.abi.json", "constructor", "{}", None, None, false)
            .unwrap();

        let exit_code = call_contract(
            &name,
            Some("10000000000"), //account balance 10T
            MsgInfo {
                balance: Some("1000000000"), // msg balance = 1T
                src: None,
                now: 1,
                bounced: false,
                body: Some(body.into())
            },
            None,
            None,
            None,
            None,
            Some(|_, _| {}),
            TraceLevel::Full,
            debug_map_filename,
        );
        assert_eq!(exit_code, 0);
    }
}
