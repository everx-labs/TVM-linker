use std::collections::HashMap;
use ton_block::Serializable;
use tvm::assembler::compile_code;
use tvm::stack::dictionary::{HashmapE, HashmapType};
use tvm::stack::{BuilderData, SliceData};

pub fn build_hashmap<K>(pairs: &[(K, SliceData)]) -> SliceData 
where 
    K: Default + Serializable {
    let bit_len = SliceData::from(K::default().write_to_new_cell().unwrap()).remaining_bits();
    let mut dict = HashmapE::with_bit_len(bit_len);
    for pair in pairs.iter() {
        dict.set(pair.0.write_to_new_cell().unwrap().into(), pair.1.clone()).unwrap();
    }
    dict.get_data()
}

pub fn prepare_methods<T>(methods: &HashMap<T, String>) -> Result<SliceData, String>
where T: Clone + Default + Eq + std::fmt::Display + Serializable + std::hash::Hash {
    let method_vec: Vec<_> = 
        methods
            .iter()
            .map(|pair| (pair.0.clone(), compile_code(&pair.1).map_err(|e| format!("func {}: compilation failed: {}", &pair.0, e)).expect("error")))
            .collect(); 
    Ok(build_hashmap(&method_vec[..]))
}

#[allow(dead_code)]
pub fn attach_method<T>(dict: SliceData, method: (T, SliceData)) -> SliceData
where T: Default + Serializable {
    let key_slice: SliceData = method.0.write_to_new_cell().unwrap().into();
    let bit_len = key_slice.remaining_bits();
    let mut dict = HashmapE::with_data(bit_len, dict);
    dict.set(key_slice, method.1).unwrap();
    dict.get_data()
}

/// Compiles authentication function to slice and builds and 
/// attaches auth dictionary as ref0
pub fn prepare_auth_method<K>(method: &str, map: &HashMap<K, bool>) -> SliceData 
where 
    K: Clone + Default + Eq + Serializable + std::hash::Hash {
    let mut method = BuilderData::from(&compile_code(method).unwrap().cell());
    let key_val_vec: Vec<_> = map
        .iter()
        .map(|pair| (pair.0.clone(), pair.1.clone().write_to_new_cell().unwrap().into()))
        .collect(); 
    if key_val_vec.len() == 0 {
        BuilderData::new().into()
    } else {
        let auth_dict = build_hashmap(&key_val_vec);
        method.checked_append_reference(auth_dict.cell()).unwrap();
        method.into()
    }
}