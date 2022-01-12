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
use crate::printer::*;
use program::load_from_file;
use std::str::FromStr;
use std::io::Cursor;
use std::str;
use std::io::prelude::*;
use std::fs::File;
extern crate hex;
use ton_block::*;
use ton_types::types::AccountId;
use ton_types::cells_serialization::{BocSerialiseMode, BagOfCells, deserialize_cells_tree_ex};
use ton_types::{SliceData, BuilderData};

pub fn load_stateinit(file_name: &str) -> Result<(SliceData, Vec<u8>), String> {
    let mut orig_bytes = Vec::new();
    let mut f = File::open(file_name)
        .map_err(|e| format!("Failed to open file {}: {}", file_name, e))?;
    f.read_to_end(&mut orig_bytes)
        .map_err(|e| format!("Failed to read file data: {}", e))?;

    let mut cur = Cursor::new(orig_bytes.clone());
    let (root_cells, _mode, _x, _y) = deserialize_cells_tree_ex(&mut cur)
        .map_err(|e| format!("Failed to deserialize BOC: {}", e))?;
    let mut root = root_cells[0].clone();
    if root.references_count() == 2 { // append empty library cell
        let mut adjusted_cell = BuilderData::from(root);
        adjusted_cell.append_reference(BuilderData::default());
        root = adjusted_cell.into_cell()
            .map_err(|e| format!("Failed to serialize cell: {}", e))?;
    }
    Ok((SliceData::from(root), orig_bytes))
}

pub fn decode_boc(filename: &str, is_tvc: bool) -> Result<(), String> {
    let (mut root_slice, orig_bytes) = load_stateinit(filename)?;

    println!("Encoded: {}\n", hex::encode(orig_bytes));
    if is_tvc {
        let state = StateInit::construct_from(&mut root_slice).map_err(|e| format!("Failed to read state_init from the slice: {}", e))?;
        println!("Decoded:\n{}", state_init_printer(&state));
    } else {
        let msg = Message::construct_from(&mut root_slice).map_err(|e| format!("Failed to read message from the slice: {}", e))?;
        println!("Decoded:\n{}", msg_printer(&msg));
    }
    Ok(())
}

pub fn compile_message(
    address_str: &str, 
    wc: Option<&str>, 
    body: Option<SliceData>, 
    pack_code: bool, 
    suffix: &str,
) -> std::result::Result<(), String> {
    let wc = match wc {
        Some(w) => i8::from_str_radix(w, 10).map_err(|_| "workchain id is not a valid int8 number".to_string())?,
        None => -1,
    };
    println!("contract address {}", address_str);
    let dest_address = MsgAddressInt::with_standart(
        None, 
        wc, 
        AccountId::from_str(address_str).map_err(|_| "input string is not a valid address".to_string())?
    ).unwrap();

    let state = if pack_code { Some(load_from_file(&format!("{}.tvc", address_str))?) } else { None };
    
    let mut msg_hdr = ExternalInboundMessageHeader::default();
    msg_hdr.dst = dest_address;
    let mut msg = Message::with_ext_in_header(msg_hdr);
    *msg.state_init_mut() = state;
    *msg.body_mut() = body;

    let root_cell = msg.serialize().map_err(|e| format!("failed to pack msg in cell: {}", e))?;
    let boc = BagOfCells::with_root(&root_cell);
    let mut bytes = Vec::new();
    let mode = BocSerialiseMode::Generic { index: false, crc: true, cache_bits: false, flags: 0 };
    boc.write_to_ex(&mut bytes, mode, None, Some(4)).unwrap();

    println!("Encoded msg: {}", hex::encode(&bytes));

    let output_file_name = address_str.get(0..8).unwrap().to_string() + suffix;
    let mut f = File::create(&output_file_name).map_err(|_| "Unable to create msg file".to_string())?;
    f.write_all(&bytes).map_err(|_| format!("Unable to write_data to msg file {}", output_file_name))?;

    println!("boc file created: {}", output_file_name);
    Ok(())
}
