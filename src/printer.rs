/*
 * Copyright 2018-2022 TON DEV SOLUTIONS LTD.
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
use failure::format_err;
use ton_block::*;
use ton_types::{cells_serialization::serialize_tree_of_cells};
use ton_types::{BuilderData, Cell, Result};

fn get_version(root: &Cell) -> Result<String> {
    let cell1 = root.reference(0).map_err(|e| format_err!("not found ({})", e))?;
    let cell2 = cell1.reference(1).map_err(|e| format_err!("not found ({})", e))?;
    let bytes = cell2.data();
    match String::from_utf8(bytes.to_vec()) {
        Ok(string) => if string.is_empty() { Ok("<empty>".to_string()) } else { Ok(string) },
        Err(e) => Err(format_err!("decoding failed ({})", e))
    }
}

pub fn get_version_mycode_aware(root: Option<&Cell>) -> Result<String> {
    let root = root.ok_or_else(|| format_err!("not found (empty root)"))?;
    match get_version(root) {
        Ok(res) => Ok(res),
        Err(_) => {
            let root = root.reference(1)?;
            get_version(&root)
        }
    }
}

pub fn state_init_printer(state: &StateInit) -> String {
    format!("StateInit\n split_depth: {}\n special: {}\n data: {}\n code: {}\n code_hash: {}\n data_hash: {}\n code_depth: {}\n data_depth: {}\n version: {}\n lib:  {}\n",
        state.split_depth.as_ref().map_or("None".to_string(), |x| x.as_u32().to_string()),
        state.special.as_ref().map_or("None".to_string(), ToString::to_string),
        tree_of_cells_into_base64(state.data.as_ref()),
        tree_of_cells_into_base64(state.code.as_ref()),
        state.code.as_ref().map(|code| code.repr_hash().to_hex_string()).unwrap_or_else(|| "None".to_string()),
        state.data.as_ref().map(|code| code.repr_hash().to_hex_string()).unwrap_or_else(|| "None".to_string()),
        state.code.as_ref().map(|code| code.repr_depth().to_string()).unwrap_or_else(|| "None".to_string()),
        state.data.as_ref().map(|code| code.repr_depth().to_string()).unwrap_or_else(|| "None".to_string()),
        get_version_mycode_aware(state.code.as_ref()).unwrap_or_else(|_| "None".to_string()),
        tree_of_cells_into_base64(state.library.root()),
    )
}

pub fn tree_of_cells_into_base64(root_cell: Option<&Cell>) -> String {
    match root_cell {
        Some(cell) => {
            let mut bytes = Vec::new();
            match serialize_tree_of_cells(cell, &mut bytes) {
                Ok(()) => base64::encode(&bytes),
                Err(_) => "None".to_string()
            }
        }
        None => "None".to_string()
    }
}

pub fn msg_printer(msg: &Message) -> Result<String> {
    let mut b = BuilderData::new();
    msg.write_to(&mut b)?;
    let mut bytes = Vec::new();
    serialize_tree_of_cells(&b.into_cell()?, &mut bytes)?;
    Ok(format!("message header\n{}init  : {}\nbody  : {}\nbody_hex: {}\nbody_base64: {}\nboc_base64: {}\n",
        print_msg_header(msg.header()),
        msg.state_init().as_ref().map(|x| {
            state_init_printer(x)
        }).unwrap_or_else(|| "None".to_string()),
        match msg.body() {
            Some(slice) => format!("{:.2}", slice.into_cell()),
            None => "None".to_string(),
        },
        msg.body()
            .map(|b| hex::encode(b.get_bytestring(0)))
            .unwrap_or_else(|| "None".to_string()),
        tree_of_cells_into_base64(
            msg.body()
                .map(|slice| slice.into_cell())
                .as_ref(),
        ),
        base64::encode(&bytes),
    ))
}

fn print_msg_header(header: &CommonMsgInfo) -> String {
    match header {
        CommonMsgInfo::IntMsgInfo(header) => {
            format!("   ihr_disabled: {}\n", header.ihr_disabled) +
            &format!("   bounce      : {}\n", header.bounce) +
            &format!("   bounced     : {}\n", header.bounced) +
            &format!("   source      : {}\n", &header.src) +
            &format!("   destination : {}\n", &header.dst) +
            &format!("   value       : {}\n", print_cc(&header.value)) +
            &format!("   ihr_fee     : {}\n", print_grams(&header.ihr_fee)) +
            &format!("   fwd_fee     : {}\n", print_grams(&header.fwd_fee)) +
            &format!("   created_lt  : {}\n", header.created_lt) +
            &format!("   created_at  : {}\n", header.created_at)
        },
        CommonMsgInfo::ExtInMsgInfo(header) => {
            format!( "   source      : {}\n", &header.src) +
            &format!("   destination : {}\n", &header.dst) +
            &format!("   import_fee  : {}\n", print_grams(&header.import_fee))
        },
        CommonMsgInfo::ExtOutMsgInfo(header) => {
            format!( "   source      : {}\n", &header.src) +
            &format!("   destination : {}\n", &header.dst) +
            &format!("   created_lt  : {}\n", header.created_lt) +
            &format!("   created_at  : {}\n", header.created_at)
        }
    }
}

fn print_grams(grams: &Grams) -> String {
    grams.to_string()
}

fn print_cc(cc: &CurrencyCollection) -> String {
    let mut result = print_grams(&cc.grams);
    if !cc.other.is_empty() {
        result += " other: {";
        cc.other.iterate_with_keys(|key: u32, value| {
            result += &format!(" \"{}\": \"{}\",", key, value);
            Ok(true)
        }).ok();
        result.pop(); // remove extra comma
        result += " }";
    }
    result
}

#[test]
fn check_output_for_money() {
    let mut cc = CurrencyCollection::with_grams(u64::MAX >> 8);
    assert_eq!(print_grams(&cc.grams), "72057594037927935");
    assert_eq!(print_cc(&cc), "72057594037927935");
    cc.set_other(12, 125).unwrap();
    cc.set_other_ex(17, &VarUInteger32::from_two_u128(1, 1900).unwrap()).unwrap();
    cc.set_other_ex(u32::MAX, &VarUInteger32::from_two_u128(u128::MAX >> 8, u128::MAX).unwrap()).unwrap();
    assert_eq!(print_grams(&cc.grams), "72057594037927935");
    assert_eq!(print_cc(&cc), r#"72057594037927935 other: { "12": "125", "17": "340282366920938463463374607431768213356", "4294967295": "452312848583266388373324160190187140051835877600158453279131187530910662655" }"#);
}
