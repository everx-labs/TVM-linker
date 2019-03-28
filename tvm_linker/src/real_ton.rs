use std::str::FromStr;
use std::sync::Arc;

extern crate hex;

use tvm::types::AccountAddress;
use tvm::cells_serialization::{ BocSerialiseMode, BagOfCells, deserialize_cells_tree, deserialize_cells_tree_ex };
use tvm::stack::BuilderData;
use tvm::stack::SliceData;
use tvm::stack::CellData;
use tvm::bitstring::Bitstring;

use ton_block::{ Message, ExternalInboundMessageHeader, MsgAddrStd, MsgAddressInt, Serializable };

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

    let mut root_cell = SliceData::from(Arc::<CellData>::from(msg.write_to_new_cell().unwrap()));

    let mode = BocSerialiseMode::Generic { index: false, crc: true, cache_bits: false, flags: 0 };
    let boc = BagOfCells::with_roots([root_cell].to_vec());
    let mut bytes = Vec::with_capacity(100);
    boc.write_to_ex(&mut bytes, mode.clone(), None, Some(4)).unwrap();

    let bytes_len = bytes.len();
    println!("Encoded message: {}, len = {}", hex::encode(bytes), bytes_len);
}
/*
fn test_real_ton_mgs2() {

    // Compatibility checking
    
    let input = "B5EE9C7241040201000000002D00024D89FE9CAEAC6436A6402388447058A8F0AD3A437A2BDE67ABDBC9CFF84A0811524DA4000000091C010100005E5050FA";
	let orig_bytes = hex::decode(input).expect("Decoding failed");
	let mut cur = Cursor::new(orig_bytes.clone());
	let (root_cells, mode) = deserialize_cells_tree_ex(&mut cur).expect("Error deserialising BOC");

	let root_cells_vec : Vec<SliceData> = root_cells.iter().map(|c| SliceData::from(c)).collect();

    let root_cell = root_cells_vec[0].clone();

	let boc = BagOfCells::with_roots(root_cells_vec);
	let mut bytes = Vec::with_capacity(orig_bytes.len());
	boc.write_to_ex(&mut bytes, mode.clone(), None, Some(4)).unwrap();

	assert_eq!(orig_bytes, bytes);


    // example: deserialize message and including custom code into

	let mut msg = Message::default();
	msg.read_from(&mut SliceData::from(root_cell));

    println!("Message = {:?}", msg);


    let code = "
        ; s0 - function selector
        ; s1 - body slice
        IFNOTRET
        DUP
        SEMPTY
        IFRET
        ; load SmartContractInfo
        PUSHCTR c5
        CTOS
        LDREF
        DROP
        CTOS
        ; skip all until block unixtime
        PUSHINT 64
        SDSKIPFIRST
        LDU 32      ; load unixtime
        PUSHINT 64
        SDSKIPFIRST ; skip all until tr logical time
        LDU 64
        DROP        ; drop remaining info
        INC         ; increase logical time by 1
        PUSH s2     ; body to top
        PUSHINT 96  ; internal header in body, cut unixtime and lt
        SDSKIPLAST
        NEWC
        STSLICE
        STU 64         ; store tr lt
        STU 32         ; store unixtime
        STSLICECONST 0 ; no init
        STSLICECONST 0 ; body (Either X)
        ENDC
        SENDMSG
        ";
    
    let code_cell = assembler::Engine::<assembler::CodePage0>::new()
        .compile(code)
        .unwrap()
        .finalize()
        .cell();

    let mut state_init = StateInit::default();        
    state_init.set_code(code_cell);        
    msg.init = Some(state_init);

    // example: serialize message struct into binary boc file

    let root_cell = SliceData::from(Arc::<CellData>::from(msg.write_to_new_cell().unwrap()));
	let boc = BagOfCells::with_root(root_cell);
	let mut bytes = Vec::new();
	boc.write_to_ex(&mut bytes, mode, None, Some(4)).unwrap();
    println!("{}", hex::encode(bytes));
*/