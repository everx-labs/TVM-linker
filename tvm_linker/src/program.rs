use ed25519_dalek::{Keypair, PUBLIC_KEY_LENGTH};
use std::io::Write;
use std::sync::Arc;
use methdict::*;
use ton_block::*;
use tvm::assembler::compile_code;
use tvm::cells_serialization::{BagOfCells};
use tvm::stack::*;
use tvm::stack::dictionary::{HashmapE, HashmapType};
use parser::{ptr_to_builder, ParseEngine};

const AUTH_METHOD_NAME: &'static str = ":authenticate";

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

    pub fn data(&self) -> Result<Arc<CellData>, String> {
        let bytes = 
            if let Some(ref pair) = self.keypair {
                pair.public.to_bytes()
            } else {
                [0u8; PUBLIC_KEY_LENGTH]
            };
            
        let mut data_dict = HashmapE::with_data(64, self.engine.data());
        data_dict.set(
            ptr_to_builder(self.engine.persistent_base)?.into(),
            BuilderData::with_raw(bytes.to_vec(), PUBLIC_KEY_LENGTH * 8).into(),
        ).unwrap();
        Ok(data_dict.get_data().into_cell())
    }

    pub fn entry(&self) -> &str {
        self.engine.entry()
    }

    pub fn method_dict(&self) -> Result<SliceData, String> {
        let mut method_dict = HashmapE::with_data(32, self.build_toplevel_dict()?);
        let methods = prepare_methods(&self.engine.globals())?;
        let key = 1i32.write_to_new_cell().unwrap();
        method_dict.set(key.into(), methods).unwrap();
        Ok(method_dict.get_data())
    }
   
    pub fn compile_to_file(&self) -> Result<String, String> {
        let mut state = StateInit::default();
        state.set_code(self.compile_asm()?.cell());
        state.set_data(self.data()?.into());

        save_to_file(state, None)
    }

    pub fn compile_asm(&self) -> Result<SliceData, String> {
        let mut bytecode = compile_code(self.engine.entry()).map_err(|e| format!("Compilation failed: {}", e))?;
        bytecode.append_reference(self.method_dict()?);
        Ok(bytecode)
    }

    fn build_toplevel_dict(&self) -> Result<SliceData, String> {
        let mut dict = prepare_methods(self.engine.internals())?;
        
        if let Some(auth) = self.engine.internal_by_name(AUTH_METHOD_NAME) {
            let auth_method = prepare_auth_method(
                &auth.1,
                self.engine.signed()
            );
            dict = attach_method(dict, (auth.0, auth_method));
        }
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

pub fn save_to_file(state: StateInit, name: Option<&str>) -> Result<String, String> {
    let root_slice = SliceData::from(
        state.write_to_new_cell().map_err(|e| format!("Serialization failed: {}", e))?
    );
    let mut buffer = vec![]; 
    BagOfCells::with_root(root_slice).write_to(&mut buffer, false)
        .map_err(|e| format!("BOC failed: {}", e))?;

    let mut print_filename = false;
    let file_name = if name.is_some() {
        format!("{}.tvc", name.unwrap())
    } else {
        print_filename = true;
        format!("{:x}.tvc", state.hash().unwrap())
    };

    let mut file = std::fs::File::create(&file_name).unwrap();
    file.write_all(&buffer).map_err(|e| format!("Write to file failed: {}", e))?;
    if print_filename {
        println! ("Saved contract to file {}", &file_name);
    }
    Ok(file_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use testcall::perform_contract_call;

    #[test]
    fn test_pbank_call() {
        let mut parser = ParseEngine::new();
        let pbank_file = File::open("./tests/pbank.s").unwrap();
        let test_file = File::open("./stdlib.tvm").unwrap();
        parser.parse(pbank_file, vec![test_file]).unwrap();
        let prog = Program::new(parser);
        let body = {
            let buf = hex::decode("002E695F78").unwrap();
            let buf_bits = buf.len() * 8;
            Some(BuilderData::with_raw(buf, buf_bits).into())
        };
        let contract_file = prog.compile_to_file().unwrap();
        let name = contract_file.split('.').next().unwrap();

        assert_eq!(perform_contract_call(name, body, None, false, false, None), 0);
    }

    #[test]
    fn test_sum_global_array() {
        let mut parser = ParseEngine::new();
        let pbank_file = File::open("./tests/sum-global-array.s").unwrap();
        let test_file = File::open("./stdlib_c.tvm").unwrap();
        assert_eq!(parser.parse(pbank_file, vec![test_file]), ok!());
        let prog = Program::new(parser);
        let body = {
            let buf = hex::decode("000D6E4079").unwrap();
            let buf_bits = buf.len() * 8;
            Some(BuilderData::with_raw(buf, buf_bits).into())
        };
        let contract_file = prog.compile_to_file().unwrap();
        let name = contract_file.split('.').next().unwrap();

        assert_eq!(perform_contract_call(name, body, None, false, false, None), 0);
    }

}