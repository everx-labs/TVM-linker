use std::collections::HashMap;
use tvm::bitstring::Bitstring;

pub struct Program {
    pub xrefs: HashMap<String,i32>,
    pub code: HashMap<i32,String>,
    pub data: Bitstring
}

impl Program {
    pub fn new() -> Self {
        Program { 
            xrefs: HashMap::new(), 
            code: HashMap::new(), 
            data: Bitstring::default(), 
        }
    }
}
