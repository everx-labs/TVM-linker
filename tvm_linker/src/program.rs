use ed25519_dalek::Keypair;
use std::io::Write;
use std::sync::Arc;
use methdict::*;
use ton_block::*;
use tvm::assembler::compile_code;
use tvm::cells_serialization::{BagOfCells};
use tvm::stack::*;
use tvm::stack::dictionary::{HashmapE, HashmapType};
use parser::ParseEngine;

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
        let mut data = self.engine.data().clone();
        if let Some(ref pair) = self.keypair {
            let bytes = pair.public.to_bytes();
            data.append_raw(&bytes, bytes.len() * 8)
                .map_err(|e| format!("{}", e))?;
        }
        Ok(data.into())
    }

    pub fn entry(&self) -> &str {
        self.engine.entry()
    }

    pub fn method_dict(&self) -> SliceData {
        let mut method_dict = HashmapE::with_data(32, self.build_toplevel_dict());
        let methods = prepare_methods(self.engine.generals());
        let key = 1i32.write_to_new_cell().unwrap();
        method_dict.set(key.into(), methods).unwrap();
        method_dict.get_data()
    }
   
    pub fn compile_to_file(&self) -> Result<(), String> {
        let mut state = StateInit::default();
        state.set_code(self.compile_asm()?.cell());
        state.set_data(self.data()?.into());

        let root_slice = SliceData::from(
            state.write_to_new_cell().map_err(|e| format!("Serialization failed: {}", e))?
        );
        let mut buffer = vec![]; 
        BagOfCells::with_root(root_slice).write_to(&mut buffer, false)
            .map_err(|e| format!("BOC failed: {}", e))?;

        let mut file = std::fs::File::create(&format!("{:x}.tvc", state.hash().unwrap())).unwrap();
        file.write_all(&buffer).map_err(|e| format!("Write to file failed: {}", e))?;
        ok!()
    }

    pub fn compile_asm(&self) -> Result<SliceData, String> {
        let mut bytecode = compile_code(self.engine.entry()).map_err(|e| format!("Compilation failed: {}", e))?;
        bytecode.append_reference(self.method_dict());
        Ok(bytecode)
    }

    fn build_toplevel_dict(&self) -> SliceData {
        let auth_method = prepare_auth_method(&self.engine.internals().get(&2).unwrap(), self.engine.signed());
        let dict = prepare_methods(self.engine.internals());
        attach_method(dict, (2, auth_method))
    }

    pub fn debug_print(&self) {
        self.engine.debug_print();
        let line = "--------------------------";
        println! ("Dictionary of methods:\n{}\n{}", line, self.method_dict());
    }
}



