use std::io::Write;
use std::fs::File;
use std::collections::HashMap;
use ton_types::dictionary::HashmapE;
use serde::{Deserialize, Serialize};
use ton_block::{Serializable, StateInit};
use ton_types::UInt256;
use ton_labs_assembler::DbgInfo;

pub struct ContractDebugInfo {
    pub hash2function: HashMap<UInt256, String>,
    pub map: DbgInfo
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DebugInfoFunction {
    pub id: i64,  // actually either i32 or u32.
    pub name: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DebugInfo {
    pub internals: Vec<DebugInfoFunction>,
    pub publics: Vec<DebugInfoFunction>,
    pub privates: Vec<DebugInfoFunction>,
}

impl DebugInfo {
    pub fn new() -> Self {
        DebugInfo { internals: vec![], publics: vec![], privates: vec![] }
    }
}


pub fn save_debug_info(
    info: DebugInfo,
    filename: String
) {
    let s = serde_json::to_string_pretty(&info).unwrap();
    let mut f = std::fs::File::create(filename).unwrap();
    write!(f, "{}", s).unwrap();
}

pub fn load_debug_info(
    state_init: &StateInit,
    filename: String,
    filename_map: String,
) -> Option<ContractDebugInfo> {

    // println!("---- load_debug_info ----");

    let debug_map = match File::open(filename_map) {
        Ok(file) => serde_json::from_reader(file).unwrap(),
        Err(_) => DbgInfo::new()
    };

    let mut hash2function = HashMap::new();

    let debug_info_str = std::fs::read_to_string(filename);
    if debug_info_str.is_err() {
        return Some(ContractDebugInfo { hash2function: hash2function, map: debug_map });
    }
    let debug_info_json : DebugInfo = serde_json::from_str(&debug_info_str.unwrap()).unwrap();

    // println!("{:?}", debug_info_json);

    let root_cell = state_init.code.as_ref().unwrap();
    let dict1 = HashmapE::with_hashmap(32, Some(root_cell.reference(0).unwrap()));
    let dict2 = HashmapE::with_hashmap(32, Some(root_cell.reference(1).unwrap().reference(0).unwrap()));

    for func in debug_info_json.internals.iter() {
        let id = func.id as i32;
        let key = id.clone().write_to_new_cell().unwrap().into();
        let val = dict1.get(key).unwrap();
        if val.is_some() {
            let val = val.unwrap();
            let hash = val.cell().repr_hash();
            hash2function.insert(hash, func.name.clone());
        }
    }
    
    for func in debug_info_json.publics.iter() {
        let id = &(func.id as u32);
        let key = id.clone().write_to_new_cell().unwrap().into();
        let val = dict1.get(key).unwrap();
        if val.is_some() {
            let val = val.unwrap();
            let hash = val.cell().repr_hash();
            hash2function.insert(hash, func.name.clone());
        }
    }

    for func in debug_info_json.privates.iter() {
        let id = &(func.id as u32);
        let key = id.clone().write_to_new_cell().unwrap().into();
        let val = dict2.get(key).unwrap();
        if val.is_some() {
            let val = val.unwrap();
            let hash = val.cell().repr_hash();
            hash2function.insert(hash, func.name.clone());
        }
    }

    hash2function.insert(root_cell.repr_hash(), "selector".to_owned());

    Some(ContractDebugInfo { hash2function: hash2function, map: debug_map })
}

