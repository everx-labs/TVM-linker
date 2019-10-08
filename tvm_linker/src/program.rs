use base64::encode;
use crc16::*;
use ed25519_dalek::{Keypair, PUBLIC_KEY_LENGTH};
use std::io::Cursor;
use std::io::Write;
use std::sync::Arc;
use methdict::*;
use tvm::block::*;
use tvm::assembler::compile_code;
use tvm::cells_serialization::{BagOfCells, deserialize_cells_tree};
use tvm::stack::*;
use tvm::stack::dictionary::{HashmapE, HashmapType};
use tvm::assembler::CompileError;
use parser::{ptr_to_builder, ParseEngine};

pub struct Program {
    engine: ParseEngine,
    keypair: Option<Keypair>,
}

impl Program {
    pub fn new(parser: ParseEngine) -> Self {
        Program { 
            engine: parser,
            keypair: None,
        }
    }

    pub fn set_keypair(&mut self, pair: Keypair) {
        self.keypair = Some(pair);        
    }

    pub fn data(&self) -> Result<SliceData, String> {
        let bytes = 
            if let Some(ref pair) = self.keypair {
                pair.public.to_bytes()
            } else {
                [0u8; PUBLIC_KEY_LENGTH]
            };
            
        let mut data_dict = HashmapE::with_hashmap(64, self.engine.data().as_ref());
        data_dict.set(
            ptr_to_builder(self.engine.persistent_base)?.into(),
            &BuilderData::with_raw(bytes.to_vec(), PUBLIC_KEY_LENGTH * 8)
                .map_err(|e| format!("failed to pack pubkey to data dictionary: {}", e))?
                .into(),
        ).unwrap();
        let mut data_cell = BuilderData::new();
        data_cell
            .append_bit_one().unwrap()
            .checked_append_reference(data_dict.data().unwrap()).unwrap();
        Ok(data_cell.into())
    }

    #[allow(dead_code)]
    pub fn entry(&self) -> &str {
        self.engine.entry()
    }

    pub fn method_dict(&self) -> Result<SliceData, String> {
        let mut method_dict = HashmapE::with_hashmap(32, self.build_toplevel_dict()?.as_ref());
        let methods = prepare_methods(&self.engine.globals())
            .map_err(|e| {
                let name = self.engine.global_name(e.0).unwrap();
                let code = self.engine.global_by_name(&name).unwrap().1;
                format_compilation_error_string(e.1, &name, &code)
            })?;

        if methods.is_some() {
            method_dict.setref(
                1i32.write_to_new_cell().unwrap().into(), 
                &methods.unwrap()
            )
            .map_err(|e| format!("cannot setref to top level dictionary: {}", e))?;
        }
        
        //convert Hashmap to HashmapE
        let mut dict_cell = BuilderData::new();
        match method_dict.data() {
            Some(cell) => {
                dict_cell.append_bit_one().unwrap();
                dict_cell.checked_append_reference(cell).unwrap()
            },
            None => dict_cell.append_bit_zero().unwrap(),
        };
        Ok(dict_cell.into())
    }
   
    pub fn compile_to_file(&self, wc: i8) -> Result<String, String> {
        save_to_file(self.compile_to_state()?, None, wc)
    }

    pub fn compile_to_state(&self) -> Result<StateInit, String> {
        let mut state = StateInit::default();
        state.set_code(self.compile_asm()?.cell().clone());
        state.set_data(self.data()?.cell().clone());
        Ok(state)
    }

    pub fn compile_asm(&self) -> Result<SliceData, String> {
        let mut bytecode = compile_code(self.engine.entry())
            .map_err(|e| format_compilation_error_string(e, "selector", self.engine.entry()) )?;
        bytecode.append_reference(self.method_dict()?);
        Ok(bytecode)
    }

    fn build_toplevel_dict(&self) -> Result<Option<Arc<CellData>>, String> {
        let dict = prepare_methods(self.engine.internals())
            .map_err(|e| {
                let name = self.engine.internal_name(e.0).unwrap();
                let code = self.engine.internal_by_name(&name).unwrap().1;
                format_compilation_error_string(e.1, &name, &code)
            })?;
        
        Ok(dict)
    }

    pub fn debug_print(&self) {
        self.engine.debug_print();
        let line = "--------------------------";
        if let Ok(slice) = self.method_dict() {
            println! ("Dictionary of methods:\n{}\n{}", line, slice);
        }
    }
}

pub fn save_to_file(state: StateInit, name: Option<&str>, wc: i8) -> Result<String, String> {
    let root_cell = state.write_to_new_cell()
        .map_err(|e| format!("Serialization failed: {}", e))?
        .into();
    let mut buffer = vec![]; 
    BagOfCells::with_root(&root_cell).write_to(&mut buffer, false)
        .map_err(|e| format!("BOC failed: {}", e))?;

    let mut print_filename = false;
    let address = state.hash().unwrap();
    let file_name = if name.is_some() {
        format!("{}.tvc", name.unwrap())
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

fn format_compilation_error_string(err: CompileError, func_name: &str, func_code: &str) -> String {
    let line_num = match err {
        CompileError::Syntax(position @ _, _) => position.line,
        CompileError::UnknownOperation(position @ _, _) => position.line,
        CompileError::Operation(position @ _, _, _) => position.line,
    };
    format!("compilation failed: \"{}\":{}:\"{}\"", 
        func_name,
        err,
        func_code.lines().nth(line_num - 1).unwrap(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use testcall::perform_contract_call;

    #[test]
    fn test_sum_global_array() {
        let mut parser = ParseEngine::new();
        let pbank_file = File::open("./tests/sum-global-array.s").unwrap();
        let test_file = File::open("./stdlib_c.tvm").unwrap();
        assert_eq!(parser.parse(pbank_file, vec![test_file], None), ok!());
        let prog = Program::new(parser);
        let body = {
            let buf = hex::decode("000D6E4079").unwrap();
            let buf_bits = buf.len() * 8;
            Some(BuilderData::with_raw(buf, buf_bits).unwrap().into())
        };
        let contract_file = prog.compile_to_file(0).unwrap();
        let name = contract_file.split('.').next().unwrap();

        assert_eq!(perform_contract_call(name, body, Some(None), false, false, None, None), 0);
    }

    #[test]
    fn test_comm_var_addresses() {
        let mut parser = ParseEngine::new();
        let source = File::open("./tests/comm_test2.s").unwrap();
        let lib = File::open("./stdlib_c.tvm").unwrap();
        assert_eq!(parser.parse(source, vec![lib], None), ok!());
        let prog = Program::new(parser);
        let body = {
            let buf = hex::decode("000D6E4079").unwrap();
            let buf_bits = buf.len() * 8;
            Some(BuilderData::with_raw(buf, buf_bits).unwrap().into())
        };
        let contract_file = prog.compile_to_file(0).unwrap();
        let name = contract_file.split('.').next().unwrap();
        assert_eq!(perform_contract_call(name, body, Some(None), false, false, None, None), 0);
    }

    #[test]    
    fn test_asciz_var() {
        let mut parser = ParseEngine::new();
        let source = File::open("./tests/asci_test1.s").unwrap();
        let lib = File::open("./stdlib.tvm").unwrap();
        assert_eq!(parser.parse(source, vec![lib], None), ok!());
        let prog = Program::new(parser);
        let body = {
            let buf = hex::decode("000D6E4079").unwrap();
            let buf_bits = buf.len() * 8;
            Some(BuilderData::with_raw(buf, buf_bits).unwrap().into())
        };
        let contract_file = prog.compile_to_file(0).unwrap();
        let name = contract_file.split('.').next().unwrap();
        assert_eq!(perform_contract_call(name, body, Some(None), false, false, None, None), 0);
    }

    #[test]
    #[ignore]
    //TODO: use when stdlib will be modified to store sender key.
    fn test_sender_pubkey() {
        let mut parser = ParseEngine::new();
        let source = File::open("./tests/sign-test.s").unwrap();
        let lib = File::open("./stdlib_c.tvm").unwrap();
        assert_eq!(parser.parse(source, vec![lib], None), ok!());
        let prog = Program::new(parser);
        let body = {
            let buf = hex::decode("000D6E4079").unwrap();
            let buf_bits = buf.len() * 8;
            Some(BuilderData::with_raw(buf, buf_bits).unwrap().into())
        };
        let contract_file = prog.compile_to_file(0).unwrap();
        let name = contract_file.split('.').next().unwrap();
        
        assert_eq!(perform_contract_call(name, body, Some(Some("key1")), true, false, None, None), 0);
    }

    #[test]
    fn test_bouncable_address() {
        let addr = hex::decode("fcb91a3a3816d0f7b8c2c76108b8a9bc5a6b7a55bd79f8ab101c52db29232260").unwrap();
        let addr = calc_userfriendly_address(-1, &addr, true, true);
        assert_eq!(addr, "kf/8uRo6OBbQ97jCx2EIuKm8Wmt6Vb15+KsQHFLbKSMiYIny");
    }

    #[test]
    fn test_ticktock() {
        let mut parser = ParseEngine::new();
        let source = File::open("./tests/ticktock.code").unwrap();
        let lib = File::open("./stdlib_sol.tvm").unwrap();
        assert_eq!(parser.parse(source, vec![lib], None), ok!());
        let prog = Program::new(parser);
        let contract_file = prog.compile_to_file(-1).unwrap();
        let name = contract_file.split('.').next().unwrap();
        
        assert_eq!(perform_contract_call(name, None, None, false, false, None, Some("-1")), 0);
    }
}