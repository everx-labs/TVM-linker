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
use ed25519_dalek::{Keypair, PUBLIC_KEY_LENGTH};
use std::io::Cursor;
use std::io::Write;
use std::collections::HashMap;
use std::time::SystemTime;
use methdict::*;
use ton_block::*;
use ton_labs_assembler::compile_code;
use ton_types::cells_serialization::{BagOfCells, deserialize_cells_tree};
use ton_types::{Cell, SliceData, BuilderData, IBitstring};
use ton_types::dictionary::{HashmapE, HashmapType};
use parser::{ptr_to_builder, ParseEngine, ParseEngineResults};
use debug_info::{save_debug_info, DebugInfoFunction, DebugInfo};

pub struct Program {
    language: Option<String>,
    engine: ParseEngineResults,
    keypair: Option<Keypair>,
}

const SELECTOR_INTERNAL: &str = "
    DICTPUSHCONST 32
	DICTUGETJMP
";

impl Program {
    pub fn new(parser: ParseEngine) -> Self {
        Program {
            language: None,
            engine: ParseEngineResults::new(parser),
            keypair: None,
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
    pub fn entry(&self) -> &str {
        self.engine.entry()
    }

    pub fn internal_method_dict(&self) -> std::result::Result<Option<Cell>, String> {
        let dict = prepare_methods(&self.engine.privates())
            .map_err(|e| e.1.replace("_name_", &self.engine.global_name(e.0).unwrap()) )?;
        Ok(dict.data().map(|cell| cell.clone()))
    }
    
    fn publics_filtered(&self, remove_ctor: bool) -> HashMap<u32, String> {
        self.engine.publics().into_iter()
            .filter(|(k, _)| 
                !(remove_ctor && self.engine.global_name(*k).unwrap_or_default() == "constructor")
            ).collect()
    }
    
    fn save_debug_info(&self, filename: String) {
        let mut debug_info = DebugInfo::new();
        for pair in self.publics_filtered(false).iter() {
            let id = *pair.0;
            let name = self.engine.global_name(id).unwrap();
            debug_info.publics.push(DebugInfoFunction{id: id as i64, name: name});
        }
        for pair in self.engine.privates().iter() {
            let id = *pair.0;
            let name = self.engine.global_name(id).unwrap();
            debug_info.privates.push(DebugInfoFunction{id: id as i64, name: name});
        }
        for pair in self.engine.internals().iter() {
            let id = *pair.0;
            let name = self.engine.internal_name(id).unwrap();
            debug_info.internals.push(DebugInfoFunction{id: id as i64, name: name});
        }
        debug_info.publics.sort_by(|a, b| a.id.cmp(&b.id));
        debug_info.privates.sort_by(|a, b| a.id.cmp(&b.id));
        debug_info.internals.sort_by(|a, b| a.id.cmp(&b.id));
        save_debug_info(debug_info, filename);
    }

    pub fn public_method_dict(&self, remove_ctor: bool) -> std::result::Result<Option<Cell>, String> {
        let mut dict = prepare_methods(&self.engine.internals())
            .map_err(|e| e.1.replace("_name_", &self.engine.internal_name(e.0).unwrap()) )?;

        insert_methods(&mut dict, &self.publics_filtered(remove_ctor))
            .map_err(|e| e.1.replace("_name_", &self.engine.global_name(e.0).unwrap()) )?;

        Ok(dict.data().map(|cell| cell.clone()))
    }

    #[allow(dead_code)]
    pub fn compile_to_file(&self, wc: i8) -> std::result::Result<String, String> {
        self.compile_to_file_ex(wc, None, None, None, false, false)
    }

    pub fn compile_to_file_ex(
        &self,
        wc: i8,
        abi_file: Option<&str>,
        ctor_params: Option<&str>,
        out_file: Option<&str>,
        trace: bool,
        debug_info: bool,
    ) -> std::result::Result<String, String> {
        let mut state_init = self.compile_to_state()?;
        if let Some(ctor_params) = ctor_params {
            state_init = self.apply_constructor(state_init, abi_file.unwrap(), ctor_params, trace)?;
        }
        if debug_info {
            let debug_info_filename = format!("{}{}", abi_file.map_or("debug_info.", |a| a.trim_end_matches("abi.json")), "debug.json");
            self.save_debug_info(debug_info_filename);
        }
        save_to_file(state_init, out_file, wc)
    }

    fn apply_constructor(
        &self,
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
            trace
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

    fn compile_to_state(&self) -> std::result::Result<StateInit, String> {
        let mut state = StateInit::default();
        state.set_code(self.compile_asm(false)?);
        state.set_data(self.data()?);
        Ok(state)
    }

    fn compile_asm(&self, remove_ctor: bool) -> std::result::Result<Cell, String> {
        let mut internal_selector = compile_code(SELECTOR_INTERNAL)
            .map_err(|_| "unexpected TVM error while compiling internal selector".to_string())?;
        internal_selector.append_reference(self.internal_method_dict()?.unwrap_or_default().into());

        let mut main_selector = compile_code(self.entry())
            .map_err(|e| format_compilation_error_string(e, self.entry()).replace("_name_", "selector"))?;
        main_selector.append_reference(self.public_method_dict(remove_ctor)?.unwrap_or_default().into());
        main_selector.append_reference(internal_selector);

        Ok(main_selector.cell().clone())
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
    let cell = deserialize_cells_tree(&mut csor).unwrap().remove(0);
    StateInit::construct_from(&mut cell.into()).unwrap()
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
    use super::*;
    use std::fs::File;
    use testcall::{perform_contract_call, call_contract, MsgInfo};

    #[ignore] // due to offline constructor
    #[test]
    fn test_comm_var_addresses() {
        let source = File::open("./tests/comm_test2.s").unwrap();
        let lib = File::open("./tests/test_stdlib.tvm").unwrap();
        let parser = ParseEngine::new(source, vec![lib], None);
        assert_eq!(parser.is_ok(), true);
        let prog = Program::new(parser.unwrap());
        let body = {
            let mut b = BuilderData::new();
            b.append_u32(abi::gen_abi_id(None, "main")).unwrap();
            b.append_reference(BuilderData::new());
            Some(b.into())
        };
        let contract_file = prog.compile_to_file(0).unwrap();
        let name = contract_file.split('.').next().unwrap();
        assert_eq!(perform_contract_call(name, body, Some(None), false, false, None, None, None, None, 0, |_b,_i| {}), 0);
    }

    #[ignore] // due to offline constructor
    #[test]
    fn test_asciz_var() {
        let source = File::open("./tests/asci_test1.s").unwrap();
        let lib = File::open("./tests/test_stdlib.tvm").unwrap();
        let parser = ParseEngine::new(source, vec![lib], None);
        assert_eq!(parser.is_ok(), true);
        let prog = Program::new(parser.unwrap());
        let body = {
            let mut b = BuilderData::new();
            b.append_u32(abi::gen_abi_id(None, "main")).unwrap();
            Some(b.into())
        };
        let contract_file = prog.compile_to_file(0).unwrap();
        let name = contract_file.split('.').next().unwrap();
        assert_eq!(perform_contract_call(name, body, Some(None), false, false, None, None, None, None, 0, |_b,_i| {}), 0);
    }

    #[test]
    #[ignore]
    //TODO: use when stdlib will be modified to store sender key.
    fn test_sender_pubkey() {
        let source = File::open("./tests/sign-test.s").unwrap();
        let lib = File::open("./tests/test_stdlib_c.tvm").unwrap();
        let parser = ParseEngine::new(source, vec![lib], None);
        assert_eq!(parser.is_ok(), true);
        let prog = Program::new(parser.unwrap());
        let body = {
            let buf = hex::decode("000D6E4079").unwrap();
            let buf_bits = buf.len() * 8;
            Some(BuilderData::with_raw(buf, buf_bits).unwrap().into())
        };
        let contract_file = prog.compile_to_file(0).unwrap();
        let name = contract_file.split('.').next().unwrap();

        assert_eq!(perform_contract_call(name, body, Some(Some("key1")), false, false, None, None, None, None, 0, |_b,_i| {}), 0);
    }

    #[test]
    fn test_bouncable_address() {
        let addr = hex::decode("fcb91a3a3816d0f7b8c2c76108b8a9bc5a6b7a55bd79f8ab101c52db29232260").unwrap();
        let addr = calc_userfriendly_address(-1, &addr, true, true);
        assert_eq!(addr, "kf/8uRo6OBbQ97jCx2EIuKm8Wmt6Vb15+KsQHFLbKSMiYIny");
    }

    #[test]
    fn test_ticktock() {
        let source = File::open("./tests/ticktock.code").unwrap();
        let lib = File::open("./tests/test_stdlib_sol.tvm").unwrap();
        let parser = ParseEngine::new(source, vec![lib], None);
        assert_eq!(parser.is_ok(), true);
        let prog = Program::new(parser.unwrap());
        let contract_file = prog.compile_to_file(-1).unwrap();
        let name = contract_file.split('.').next().unwrap();

        assert_eq!(perform_contract_call(name, None, None, false, false, None, Some(-1), None, None, 0, |_b,_i| {}), 0);
    }

    #[ignore] // due to offline constructor
    #[test]
    fn test_recursive_call() {
        let lib1 = File::open("./tests/test_stdlib.tvm").unwrap();
        let source = File::open("./tests/test_recursive.code").unwrap();
        let parser = ParseEngine::new(source, vec![lib1], None);
        assert_eq!(parser.is_ok(), true);
        let prog = Program::new(parser.unwrap());
        let contract_file = prog.compile_to_file(-1).unwrap();
        let name = contract_file.split('.').next().unwrap();
        let body = {
            let mut b = BuilderData::new();
            b.append_u32(abi::gen_abi_id(None, "main")).unwrap();
            Some(b.into())
        };

        assert_eq!(perform_contract_call(name, body, Some(Some("key1")), false, false, None, None, None, None, 0, |_b,_i| {}), 0);
    }

    #[ignore] // due to offline constructor
    #[test]
    fn test_public_and_private() {
        let source = File::open("./tests/test_public.code").unwrap();
        let lib = File::open("./tests/test_stdlib.tvm").unwrap();

        let abi_str = abi::load_abi_json_string("./tests/test_public.abi.json").unwrap();
        let abi = abi::load_abi_contract(&abi_str).unwrap();

        let parser = ParseEngine::new(source, vec![lib], Some(abi_str));
        assert_eq!(parser.is_ok(), true);
        let prog = Program::new(parser.unwrap());

        let contract_file = prog.compile_to_file(0).unwrap();
        let name = contract_file.split('.').next().unwrap();
        let body1 = {
            let mut b = BuilderData::new();
            b.append_u32(abi::gen_abi_id(None, "sum")).unwrap();
            b.append_reference(BuilderData::new());
            Some(b.into())
        };

        assert_eq!(perform_contract_call(name, body1, None, false, false, None, None, None, None, 0, |_b,_i| {}), 0);

        let body2 = {
            let mut b = BuilderData::new();
            b.append_u32(abi::gen_abi_id(None, "sum_p")).unwrap();
            b.append_reference(BuilderData::new());
            Some(b.into())
        };
        assert!(perform_contract_call(name, body2, None, false, false, None, None, None, None, 0, |_b,_i| {}) != 0);

        let body3 = {
            let mut b = BuilderData::new();
            b.append_u32(abi::gen_abi_id(Some(abi), "sum2")).unwrap();
            b.append_reference(BuilderData::new());
            Some(b.into())
        };
        assert_eq!(perform_contract_call(name, body3, None, false, false, None, None, None, None, 0, |_b,_i| {}), 0);
    }

    #[test]
    fn test_call_with_gas_limit() {
        let source = File::open("./tests/Wallet.code").unwrap();
        let lib = File::open("./tests/test_stdlib_sol.tvm").unwrap();
        let abi = abi::load_abi_json_string("./tests/Wallet.abi.json").unwrap();

        let parser = ParseEngine::new(source, vec![lib], Some(abi));
        assert_eq!(parser.is_ok(), true);
        let prog = Program::new(parser.unwrap());

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
            Some(3000), // gas limit
            Some(|_, _| {}),
            false,
            String::from("")
        );
        // must equal to out of gas exception
        assert_eq!(exit_code, 13);
    }
}
