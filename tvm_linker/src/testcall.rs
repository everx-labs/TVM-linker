use keyman::KeypairManager;
use program::Program;
use sha2::Sha512;
use std::sync::Arc;
use tvm::cells_serialization::BagOfCells;
use tvm::stack::*;
use tvm::test_framework::{test_case_with_ref, Expects};
use tvm::types::AccountId;
use ton_block::*;

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

fn sign_body(body: &mut SliceData, key_file: &str) {
    let pair = KeypairManager::from_secret_file(key_file);
    let signature = 
        pair.drain().sign::<Sha512>(
            BagOfCells::with_root(body.clone()).get_repr_hash_by_index(0).unwrap().as_slice()
        ).to_bytes();
    let mut sign_builder = BuilderData::new();
    sign_builder.append_raw(&signature, signature.len() * 8).unwrap();

    let mut signed_body = BuilderData::from_slice(body);
    signed_body.prepend_reference(sign_builder);
    *body = signed_body.into();
}

pub fn perform_contract_call(prog: &Program, body: Option<Arc<CellData>>, key_file: Option<&str>) {
    let mut stack = Stack::new();
    let msg_cell = StackItem::Cell(
        create_external_inbound_msg(
            &AccountId::from([0x11; 32]), 
            body.clone(),
        ).write_to_new_cell().unwrap().into()
    );

    let mut body: SliceData = match body {
        Some(b) => b.into(),
        None => BuilderData::new().into(),
    };

    if key_file.is_some() {
        sign_body(&mut body, key_file.unwrap());
    }

    let mut info = SmartContractInfo::default();
    info.set_myself(MsgAddressInt::with_standart(None, 0, AccountId::from([0u8; 32])).unwrap());
    info.set_balance_remaining(CurrencyCollection::with_grams(10000));
    let mut builder = BuilderData::new();
    builder.append_reference(info.write_to_new_cell().unwrap());

    stack
        .push(int!(0))
        .push(int!(0))
        .push(msg_cell)
        .push(StackItem::Slice(body)) 
        .push(int!(-1));

    test_case_with_ref(
        &prog.entry(), 
        prog.method_dict(),
    )
    .with_root_data(prog.data().unwrap())
    .with_stack(stack)
    .with_ctrl(5, StackItem::Cell(builder.into()))
    .with_ctrl(6, StackItem::Cell(BuilderData::new().into()))
    .expect_success();
}