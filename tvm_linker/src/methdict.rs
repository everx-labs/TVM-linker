use std::collections::HashMap;
use tvm::block::Serializable;
use tvm::assembler::compile_code;
use tvm::stack::dictionary::{HashmapE, HashmapType};
use tvm::stack::{BuilderData, IBitstring, SliceData};
use tvm::assembler::CompileError;

pub fn build_hashmap<K>(pairs: &[(K, SliceData)]) -> SliceData 
where 
    K: Default + Serializable {
    let bit_len = SliceData::from(K::default().write_to_new_cell().unwrap()).remaining_bits();
    let mut dict = HashmapE::with_bit_len(bit_len);
    for pair in pairs.iter() {
        dict.set(pair.0.write_to_new_cell().unwrap().into(), &pair.1).unwrap();
    }
    let mut map_cell = BuilderData::new();
    if let Some(cell) = dict.data() {
        map_cell.append_bit_one().unwrap();
        map_cell.checked_append_reference(cell).unwrap();
    } else {
        map_cell.append_bit_zero().unwrap();
    }
    map_cell.into()
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
