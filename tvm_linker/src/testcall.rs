use tvm::stack::*;
use tvm::stack::dictionary::{HashmapE, HashmapType};
use tvm::test_framework::{test_case_with_ref, Expects};
use tvm::types::AccountId;
use ton_block::{
    Serializable,
    ExternalInboundMessageHeader,
    MsgAddressInt,
    Message
};
use stdlib::{_SELECTOR, INBOUND_EXTERNAL_PARSER};
use stdlib::methdict::*;
use std::sync::Arc;

pub struct TestABIContract {
    dict: SliceData,        // dictionary of methods
}

/// Constructs test contract to implement dictionary of methods
pub trait TestContractCode {
    fn new(&[(i32,String)]) -> Self;
    fn get_contract_code(&self) -> &str;
    fn get_methods(&self) -> SliceData;
}

impl TestContractCode for TestABIContract {
    fn get_contract_code(&self) -> &str {
        _SELECTOR
    }    

    fn get_methods(&self) -> SliceData {
        self.dict.clone()
    }

    fn new(raw_methods: &[(i32, String)]) -> Self {
        let dict = prepare_methods(&[
            (-1i8,  INBOUND_EXTERNAL_PARSER.to_string()),
            // (0,     MAIN),
        ]);

        let methods = prepare_methods(raw_methods);

        let key = 1i8.write_to_new_cell().unwrap();
        let mut dict = HashmapE::with_data(8, dict);
        dict.set(key.into(), methods).unwrap();
        TestABIContract { dict: dict.get_data() }
    }
}

fn create_inbound_body(a: i32, b: i32, func_id: i32) -> Arc<CellData> {
    let mut builder = BuilderData::new();
    let version: u8 = 0;
    version.write_to(&mut builder).unwrap();
    func_id.write_to(&mut builder).unwrap();
    a.write_to(&mut builder).unwrap();
    b.write_to(&mut builder).unwrap();
    builder.into()
}

fn create_external_inbound_msg(dst_addr: &AccountId, body: Arc<CellData>) -> Message {
    let mut hdr = ExternalInboundMessageHeader::default();
    hdr.dst = MsgAddressInt::with_standart(None, -1, dst_addr.clone()).unwrap();
    let mut msg = Message::with_ext_in_header(hdr);
    msg.body = Some(body.into());
    msg
}

pub fn perform_contract_call(raw_methods: &[(i32,String)], func_id: i32, _data: Option<BuilderData>) {
    let mut stack = Stack::new();
    let body_cell = create_inbound_body(0, 0, func_id);
    let msg_cell = StackItem::Cell(
        create_external_inbound_msg(
            &AccountId::from([0x11; 32]), 
            body_cell.clone()
        ).write_to_new_cell().unwrap().into()
    );
    stack
        .push(int!(0))
        .push(int!(0))
        .push(msg_cell.clone())
        .push(StackItem::Slice(SliceData::from(body_cell))) 
        .push(int!(-1));

    let contract = TestABIContract::new(raw_methods);

    test_case_with_ref(&contract.get_contract_code(), contract.get_methods())
        .with_stack(stack).expect_success();
}