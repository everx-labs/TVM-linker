use std::str::FromStr;
use std::sync::Arc;
use std::io::Cursor;
use std::str;
use std::io::prelude::*;
use std::fs::File;

extern crate hex;

use tvm::types::AccountAddress;
use tvm::cells_serialization::{ BocSerialiseMode, BagOfCells, deserialize_cells_tree_ex };
use tvm::stack::BuilderData;
use tvm::stack::SliceData;
use tvm::stack::CellData;
use tvm::assembler;
use tvm::assembler::Writer;
use tvm::bitstring::Bitstring;

use ton_block::{ Message, ExternalInboundMessageHeader, MsgAddrStd, MsgAddressInt, 
    Serializable, Deserializable, StateInit, GetRepresentationHash };

//"B5EE9C7241040301000000008A0002CF89FF86EE2B1CE113242F7CAE3511009B84F9E460D38773688AF808406AA75537991A119295932524BB029FC6BBD76D06AE732E89C14DFE4F9B1D8424BF90701E3B70E13CE43815613880BC04C254251497885DEFC82DFDE25682247A0F16269E782E0060000000100102002C20DDA4F260F8005F04ED44D0D31F30A4C8CB1FC9ED54000800000000EE5A8D0B"; 
//"B5EE9C7241040201000000006600014F89FEA71F4F9849FF1D54203B094BE356FD065FC3B0966139BFDE9DD286E755901EFA00000000980C010072427FBE50ECD496653C6CE8EF33294BF67835ED2C962454F34A37AEB2445CB03629D5A82363E7F0000000000000000000000000000047494654E8A1E917";
//"B5EE9C7241040301000000004600024789FF86EE2B1CE113242F7CAE3511009B84F9E460D38773688AF808406AA75537991A11900102002C20DDA4F260F8005F04ED44D0D31F30A4C8CB1FC9ED540008000000005A785C4E";
//"b5ee9c7241040301000000004600024789ff86ee2b1ce113242f7cae3511009b84f9e460d38773688af808406aa75537991a11900201000800000000002c20dda4f260f8005f04ed44d0d31f30a4c8cb1fc9ed540d374e22";
//  let orig_bytes = hex::decode(input).expect("Decoding failed");

pub fn decode_boc(file_name: &str) {
    let mut orig_bytes = Vec::new();

    let mut f = File::open(file_name).expect("Unable to open file");
    f.read_to_end(&mut orig_bytes).expect("Unable to read file");

    let mut cur = Cursor::new(orig_bytes.clone());
    let (root_cells, _mode, _x, _y) = deserialize_cells_tree_ex(&mut cur).expect("Error deserialising BOC");
    let root_cells_vec : Vec<SliceData> = root_cells.iter().map(|c| SliceData::from(c)).collect();
    let root_cell = root_cells_vec[0].clone();

    let mut msg = Message::default();
    msg.read_from(&mut SliceData::from(root_cell)).expect("Cannot read from message slice");

    println!("Encoded: {}", hex::encode(orig_bytes));
    println!("Decoded: {:?}", msg);
}

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

    let node_data = Bitstring::create([0,0,1,35].to_vec(),32);
    node.append_data (&node_data);

    msg.body = Some(Arc::<CellData>::from(node));

    println!("Message = {:?}", msg);

    let root_cell = SliceData::from(Arc::<CellData>::from(msg.write_to_new_cell().unwrap()));

    let mode = BocSerialiseMode::Generic { index: false, crc: true, cache_bits: false, flags: 0 };
    let boc = BagOfCells::with_roots([root_cell].to_vec());
    let mut bytes = Vec::with_capacity(100);
    boc.write_to_ex(&mut bytes, mode.clone(), None, Some(4)).unwrap();

    let bytes_len = bytes.len();
    println!("Encoded message: {}, len = {}", hex::encode(bytes), bytes_len);
}

//  let address : AccountAddress = AccountAddress::from_str ("5d76362f95fb9187ad94967ecc7347f7fc85fdbbc23722323f82e68f66f9f963").unwrap();
//  let address : AccountAddress = AccountAddress::from_str ("1b2fb433e2a10483b51540a314f8558aaf5e824c49abbbf27af0372f74829379").unwrap();

pub fn compile_real_ton (code: &str, output_file_name: &str) {
    let code_cell = assembler::Engine::<assembler::CodePage0>::new()
        .compile(code)
        .unwrap()
        .finalize()
        .cell();

    let mut state_init = StateInit::default();

    let mut node = BuilderData::new();
    let node_data = Bitstring::create(hex::decode("00000002").unwrap(),32);
    node.append_data (&node_data);

    let state_init_data = Arc::<CellData>::from(node);
    state_init.set_data (state_init_data);
    state_init.set_code(code_cell);

    let address = state_init.hash().unwrap();
    println!("Address: {}", hex::encode(address.as_slice()));

    let mut msg_hdr = ExternalInboundMessageHeader::default();
    msg_hdr.dst = MsgAddressInt::AddrStd (MsgAddrStd::with_address(None, -1, address));
    let mut msg = Message::with_ext_in_header (msg_hdr);
    msg.init = Some(state_init.clone());

    let root_cell = SliceData::from(Arc::<CellData>::from(msg.write_to_new_cell().unwrap()));
    let boc = BagOfCells::with_root(root_cell);
    let mut bytes = Vec::new();
    let mode = BocSerialiseMode::Generic { index: false, crc: true, cache_bits: false, flags: 0 };
    boc.write_to_ex(&mut bytes, mode, None, Some(4)).unwrap();

    println!("Decoded: {:?}", &msg);
    println!("Encoded: {}", hex::encode(&bytes));

    let mut f = File::create(output_file_name).expect("Unable to create file");
    f.write_all(&bytes).expect("Unable to write data");
}
