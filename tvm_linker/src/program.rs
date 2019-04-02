use std::collections::HashMap;
use tvm::bitstring::Bitstring;

pub struct Program {
    pub xrefs: HashMap<String,i32>,
    pub code: HashMap<i32,String>,
    pub data: Bitstring
}
