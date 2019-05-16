use std::collections::HashMap;
use std::io::Write;
use stdlib::methdict::*;
use stdlib::{_SELECTOR, build_default_dict};
use ton_block::*;
use tvm::assembler::compile_code;
use tvm::cells_serialization::{BagOfCells};
use tvm::stack::*;
use tvm::stack::dictionary::{HashmapE, HashmapType};

pub struct Program {
    pub xrefs: HashMap<String,i32>,
    pub code: HashMap<i32,String>,
    pub data: BuilderData,
    entry_point: String,
    is_default: bool,
}

impl Program {
    pub fn new() -> Self {
        Program { 
            xrefs: HashMap::new(), 
            code: HashMap::new(), 
            data: BuilderData::new(), 
            entry_point: _SELECTOR.to_owned(),
            is_default: true,
        }
    }

    pub fn get_method_dict(&self) -> SliceData {
        let mut method_dict = HashmapE::with_bit_len(32);
        if self.is_default {
            method_dict = HashmapE::with_data(32, build_default_dict(HashMap::new()));            
        }

        let methods: Vec<_> = self.code.iter().map(|entry| (entry.0.clone(), entry.1.clone())).collect();        
        let methods = prepare_methods(&methods);
        let key = 1i32.write_to_new_cell().unwrap();
        method_dict.set(key.into(), methods).unwrap();
        method_dict.get_data()
    }

    pub fn get_entry(&self) -> &str {
        &self.entry_point
    }

    pub fn set_entry(&mut self, entry: Option<&str>) -> Result<(), String> {
        if let Some(name) = entry {
            let id = self.xrefs.get(name).ok_or(format!("Entry point '{}' is not found in source code", name))?;
            self.entry_point = self.code.remove(&id).ok_or(format!("Code map doesn't have func with id '{}' ", id))?
        }
        ok!()
    }

    pub fn compile_to_file(&self) -> Result<(), String> {
        let mut state = StateInit::default();
        state.set_code(self.compile_asm()?.cell());
        state.set_data(self.data.clone().into());

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

    fn compile_asm(&self) -> Result<SliceData, String> {
        let mut bytecode = compile_code(&self.entry_point).map_err(|e| format!("Compilation failed: {}", e))?;
        bytecode.append_reference(self.get_method_dict());
        Ok(bytecode)
    }
}
