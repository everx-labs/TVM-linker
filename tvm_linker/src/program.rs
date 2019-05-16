use ed25519_dalek::Keypair;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use stdlib::methdict::*;
use stdlib::{_SELECTOR, build_default_dict};
use ton_block::*;
use tvm::assembler::compile_code;
use tvm::cells_serialization::{BagOfCells};
use tvm::stack::*;
use tvm::stack::dictionary::{HashmapE, HashmapType};

pub struct Program {
    pub xrefs: HashMap<String, u32>,
    pub code: HashMap<u32, String>,
    pub data: BuilderData,
    pub signed: HashMap<u32, bool>,
    entry_point: String,
    keypair: Option<Keypair>,
}

impl Program {
    pub fn new() -> Self {
        Program { 
            xrefs: HashMap::new(), 
            code: HashMap::new(), 
            data: BuilderData::new(), 
            signed: HashMap::new(),
            entry_point: String::new(),
            keypair: None,
        }
    }

    pub fn set_keypair(&mut self, pair: Keypair) {
        self.keypair = Some(pair);        
    }

    pub fn data(&self) -> Result<Arc<CellData>, String> {
        let mut data = self.data.clone();
        if let Some(ref pair) = self.keypair {
            let bytes = pair.public.to_bytes();
            data.append_raw(&bytes, bytes.len() * 8)
                .map_err(|e| format!("{}", e))?;
        }
        Ok(data.into())
    }

    pub fn method_dict(&self) -> SliceData {
        let mut method_dict = HashmapE::with_bit_len(32);
        if self.entry_point.is_empty() {
            method_dict = HashmapE::with_data(32, build_default_dict(&self.signed));            
        }

        let methods: Vec<_> = self.code.iter().map(|entry| (entry.0.clone(), entry.1.clone())).collect();        
        let methods = prepare_methods(&methods);
        let key = 1i32.write_to_new_cell().unwrap();
        method_dict.set(key.into(), methods).unwrap();
        method_dict.get_data()
    }

    pub fn entry(&self) -> &str {
        if self.entry_point.is_empty() {
            _SELECTOR
        } else {
            &self.entry_point
        }
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

    fn compile_asm(&self) -> Result<SliceData, String> {
        let mut bytecode = compile_code(&self.entry_point).map_err(|e| format!("Compilation failed: {}", e))?;
        bytecode.append_reference(self.method_dict());
        Ok(bytecode)
    }
}

pub fn calc_func_id(func_interface: &str) -> u32 {
    let mut hasher = Sha256::new();
    hasher.input(func_interface.as_bytes());
    let mut id_bytes = [0u8; 4];
    id_bytes.copy_from_slice(&hasher.result()[..4]);
    u32::from_be_bytes(id_bytes)
} 

pub fn debug_print_program(prog: &Program) {
    let line = "--------------------------";
    println!("Entry point:\n{}\n{}\n{}", line, prog.entry(), line);
    println!("Contract functions:\n{}", line);
    for (k,v) in &prog.xrefs {
        println! ("Function {:15}: id={:08X}, sign-check={:?}", k, v, prog.signed.get(v).unwrap());
    }
    for (k,v) in &prog.code {
        println! ("Function {:08X}\n{}\n{}\n{}", k, line, v, line);
    }    
    println! ("Dictionary of methods:\n{}\n{}", line, prog.method_dict());
}

