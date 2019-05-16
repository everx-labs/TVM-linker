use program::Program;
use std::sync::Arc;
use tvm::stack::*;
use tvm::test_framework::{test_case_with_ref, Expects};
use tvm::types::AccountId;
use ton_block::{
    Serializable,
    ExternalInboundMessageHeader,
    MsgAddressInt,
    Message
};

#[allow(dead_code)]
fn create_inbound_body(a: i32, b: i32, func_id: i32) -> Arc<CellData> {
    let mut builder = BuilderData::new();
    let version: u8 = 0;
    version.write_to(&mut builder).unwrap();
    func_id.write_to(&mut builder).unwrap();
    a.write_to(&mut builder).unwrap();
    b.write_to(&mut builder).unwrap();
    builder.into()
}

fn create_external_inbound_msg(dst_addr: &AccountId, body: Option<Arc<CellData>>) -> Message {
    let mut hdr = ExternalInboundMessageHeader::default();
    hdr.dst = MsgAddressInt::with_standart(None, -1, dst_addr.clone()).unwrap();
    let mut msg = Message::with_ext_in_header(hdr);
    msg.body = body;
    msg
}

pub fn perform_contract_call(prog: &Program, body: Option<Arc<CellData>>) {
    let mut stack = Stack::new();
    let msg_cell = StackItem::Cell(
        create_external_inbound_msg(
            &AccountId::from([0x11; 32]), 
            body.clone(),
        ).write_to_new_cell().unwrap().into()
    );

    let body: SliceData = match body {
        Some(b) => b.into(),
        None => BuilderData::new().into(),
    };

    stack
        .push(int!(0))
        .push(int!(0))
        .push(msg_cell)
        .push(StackItem::Slice(body)) 
        .push(int!(-1));


    test_case_with_ref(
        &prog.get_entry(), 
        prog.get_method_dict(),
    )
    .with_root_data(prog.data.clone().into())
    .with_stack(stack)
    .expect_success();
}