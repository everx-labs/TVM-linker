/*
 * Copyright (C) 2019-2021 TON Labs. All Rights Reserved.
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
use std::collections::{BTreeMap, HashMap};
use ton_block::Serializable;
use ton_labs_assembler::{compile_code_debuggable, Lines, DbgInfo};
use ton_types::{SliceData, dictionary::HashmapE};

pub fn prepare_methods<T>(
    methods: &HashMap<T, Lines>,
    adjust_entry_points: bool,
) -> Result<(HashmapE, DbgInfo), (T, String)>
where
    T: Clone + Default + Eq + std::fmt::Display + Serializable + std::hash::Hash,
{
    let bit_len = SliceData::from(T::default().serialize().unwrap()).remaining_bits();
    let mut map = HashmapE::with_bit_len(bit_len);
    let mut dbg = DbgInfo::new();
    insert_methods(&mut map, &mut dbg, methods, adjust_entry_points)?;
    Ok((map, dbg))
}

pub fn insert_methods<T>(
    map: &mut HashmapE,
    dbg: &mut DbgInfo,
    methods: &HashMap<T, Lines>,
    adjust_entry_points: bool,
) -> Result<(), (T, String)>
where
    T: Clone + Default + Eq + std::fmt::Display + Serializable + std::hash::Hash,
{
    for pair in methods.iter() {
        let key: SliceData = pair.0.clone().serialize().unwrap().into();
        let mut val = compile_code_debuggable(pair.1.clone()).map_err(|e| {
            (pair.0.clone(), e.to_string())
        })?;
        if val.0.remaining_bits() <= (1023 - (32 + 10)) { // key_length + hashmap overheads
            map.set(key.clone(), &val.0).map_err(|e| {
                (pair.0.clone(), format!("failed to set method _name_ to dictionary: {}", e))
            })?;
        } else {
            map.setref(key.clone(), &val.0.clone().into_cell()).map_err(|e| {
                (pair.0.clone(), format!("failed to set method _name_ to dictionary: {}", e))
            })?;
        }
        let id = key.clone().get_next_i32().unwrap();
        if adjust_entry_points || id < -2 || id > 0 {
            let before = val.0;
            let after = map.get(key).unwrap().unwrap();
            adjust_debug_map(&mut val.1, before, after);
        }
        dbg.append(&mut val.1)
    }
    Ok(())
}

fn adjust_debug_map(map: &mut DbgInfo, before: SliceData, after: SliceData) {
    let hash_old = before.cell().repr_hash();
    let hash_new = after.cell().repr_hash();
    let old = map.remove(&hash_old).unwrap();

    let adjustment = after.pos();
    let mut new = BTreeMap::new();
    for (k, v) in old {
        new.insert(k + adjustment, v);
    }

    map.insert(hash_new, new);
}
