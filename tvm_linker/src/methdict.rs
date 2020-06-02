/*
 * Copyright 2018-2019 TON DEV SOLUTIONS LTD.
 *
 * Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
 * this file except in compliance with the License.
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific TON DEV software governing permissions and
 * limitations under the License.
 */
use std::collections::HashMap;
use ton_block::Serializable;
use ton_vm::assembler::{compile_code, CompileError};
use ton_types::{SliceData, dictionary::HashmapE};

pub fn prepare_methods<T>(
    methods: &HashMap<T, String>,
) -> Result<HashmapE, (T, String)>
where
    T: Clone + Default + Eq + std::fmt::Display + Serializable + std::hash::Hash,
{
    let bit_len = SliceData::from(T::default().write_to_new_cell().unwrap()).remaining_bits();
    let mut map = HashmapE::with_bit_len(bit_len);
    insert_methods(&mut map, methods)?;
    Ok(map)
}

pub fn insert_methods<T>(
    map: &mut HashmapE,
    methods: &HashMap<T, String>,
) -> Result<(), (T, String)>
where
    T: Clone + Default + Eq + std::fmt::Display + Serializable + std::hash::Hash,
{
    for pair in methods.iter() {
        let key = pair.0.clone().write_to_new_cell().unwrap().into();
        let val = compile_code(&pair.1).map_err(|e| {
            (pair.0.clone(), format_compilation_error_string(e, &pair.1))
        })?;
        map.setref(key, &val.into_cell()).map_err(|e| {
            (pair.0.clone(), format!("failed to set method _name_ to dictionary: {}", e))
        })?;
    }
    Ok(())
}

pub fn format_compilation_error_string(err: CompileError, func_code: &str) -> String {
    let line_num = match err {
        CompileError::Syntax(position @ _, _) => position.line,
        CompileError::UnknownOperation(position @ _, _) => position.line,
        CompileError::Operation(position @ _, _, _) => position.line,
    };
    format!(
        "compilation failed: \"_name_\":{}:\"{}\"", 
        err,
        func_code.lines().nth(line_num - 1).unwrap(),
    )
}