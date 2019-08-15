use std::collections::HashMap;
use tvm::block::Serializable;
use tvm::assembler::compile_code;
use tvm::stack::dictionary::{HashmapE, HashmapType};
use tvm::stack::SliceData;
use tvm::assembler::CompileError;

pub fn build_hashmap<K>(pairs: &[(K, SliceData)]) -> SliceData 
where 
    K: Default + Serializable {
    let bit_len = SliceData::from(K::default().write_to_new_cell().unwrap()).remaining_bits();
    let mut dict = HashmapE::with_bit_len(bit_len);
    for pair in pairs.iter() {
        dict.set(pair.0.write_to_new_cell().unwrap().into(), &pair.1).unwrap();
    }
    dict.get_data()
}

pub fn prepare_methods<T>(methods: &HashMap<T, String>) -> Result<SliceData, (T, CompileError)>
where T: Clone + Default + Eq + std::fmt::Display + Serializable + std::hash::Hash {
    let mut method_vec = vec![]; 
    for pair in methods.iter() {
        method_vec.push(
            (
                pair.0.clone(), 
                compile_code(&pair.1).map_err(|e| (pair.0.clone(), e))?
            )
        );
    }            
    Ok(build_hashmap(&method_vec[..]))
}

#[allow(dead_code)]
pub fn attach_method<T>(dict: SliceData, method: (T, SliceData)) -> SliceData
where T: Default + Serializable {
    let key_slice: SliceData = method.0.write_to_new_cell().unwrap().into();
    let bit_len = key_slice.remaining_bits();
    let mut dict = HashmapE::with_data(bit_len, dict);
    dict.set(key_slice, &method.1).unwrap();
    dict.get_data()
}
