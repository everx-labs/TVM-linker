use program::load_from_file;
use std::str::FromStr;
use std::io::Cursor;
use std::str;
use std::io::prelude::*;
use std::fs::File;
extern crate hex;
use tvm::block::*;
use tvm::types::{AccountAddress, AccountId};
use tvm::cells_serialization::{BocSerialiseMode, BagOfCells, deserialize_cells_tree_ex };
use tvm::stack::BuilderData;
use tvm::stack::SliceData;

pub fn decode_boc(file_name: &str, is_tvc: bool) {
    let mut orig_bytes = Vec::new();

    let mut f = File::open(file_name).expect("Unable to open file");
    f.read_to_end(&mut orig_bytes).expect("Unable to read file");

    let mut cur = Cursor::new(orig_bytes.clone());
    let (root_cells, _mode, _x, _y) = deserialize_cells_tree_ex(&mut cur).expect("Error deserialising BOC");
    let root_cells_vec : Vec<SliceData> = root_cells.iter().map(|c| SliceData::from(c)).collect();
    let mut root_slice = SliceData::from(root_cells_vec[0].clone());
    
    println!("Encoded: {}\n", hex::encode(orig_bytes));
    if is_tvc {
        let state: StateInit = StateInit::construct_from(&mut root_slice).expect("cannot read state_init from slice");
        println!("Decoded: {}", &serde_json::to_string_pretty(&state).expect("cannot serialize state_init to json"));
    } else {
        let msg: Message = Message::construct_from(&mut root_slice).expect("cannot read message from slice");
        println!("Decoded: {}", &serde_json::to_string_pretty(&msg).expect("cannot serialize message to json"));
    }

}

#[allow(dead_code)]
pub fn make_boc() {
    println! ("Making real TON");
    let address : AccountAddress = AccountAddress::from_str ("4e5756321b532011c422382c5478569d21bd15ef33d5ede4e7fc250408a926d2").unwrap();
    let mut msg_hdr = ExternalInboundMessageHeader::default();
    msg_hdr.dst = MsgAddressInt::AddrStd (MsgAddrStd::with_address(None, -1, address.account_id));
    let mut msg = Message::with_ext_in_header (msg_hdr);

    let left = BuilderData::new();
    let right = BuilderData::new();
    let mut node = BuilderData::new();
    node.append_reference (left);
    node.append_reference (right);
    node.append_raw(&[0,0,1,35], 32).unwrap();

    *msg.body_mut() = Some(node.into());

    println!("Message = {:?}", msg);

    let root_cell = msg.write_to_new_cell().unwrap().into();

    let mode = BocSerialiseMode::Generic { index: false, crc: true, cache_bits: false, flags: 0 };
    let boc = BagOfCells::with_roots([&root_cell].to_vec());
    let mut bytes = Vec::with_capacity(100);
    boc.write_to_ex(&mut bytes, mode.clone(), None, Some(4)).unwrap();

    let bytes_len = bytes.len();
    println!("Encoded message: {}, len = {}", hex::encode(bytes), bytes_len);
}

pub fn compile_message(
    address_str: &str, 
    wc: Option<&str>, 
    body: Option<SliceData>, 
    pack_code: bool, 
    suffix: &str,
) -> Result<(), String> {
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

    let state = if pack_code { Some(load_from_file(&format!("{}.tvc", address_str))) } else { None };
    
    let mut msg_hdr = ExternalInboundMessageHeader::default();
    msg_hdr.dst = dest_address;
    let mut msg = Message::with_ext_in_header(msg_hdr);
    *msg.state_init_mut() = state;
    *msg.body_mut() = body;

    let root_cell = msg.write_to_new_cell().unwrap().into();
    let boc = BagOfCells::with_root(&root_cell);
    let mut bytes = Vec::new();
    let mode = BocSerialiseMode::Generic { index: false, crc: true, cache_bits: false, flags: 0 };
    boc.write_to_ex(&mut bytes, mode, None, Some(4)).unwrap();

    println!("Encoded msg: {}", hex::encode(&bytes));

    let output_file_name = address_str.get(0..8).unwrap().to_string() + suffix;
    let mut f = File::create(&output_file_name).map_err(|_| "Unable to create msg file".to_string())?;
    f.write_all(&bytes).map_err(|_| "Unable to write_data to msg file".to_string())?;

    println!("boc file created: {}", output_file_name);
    ok!()
}
