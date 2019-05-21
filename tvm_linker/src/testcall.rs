use keyman::KeypairManager;
use program::Program;
use simplelog::{SimpleLogger, Config, LevelFilter};
use sha2::Sha512;
use std::sync::Arc;
use tvm::cells_serialization::BagOfCells;
use tvm::executor::Engine;
use tvm::stack::*;
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

fn initialize_registers(code: SliceData, data: SliceData) -> SaveList {
    let mut ctrls = SaveList::new();
    let empty_cont = StackItem::Continuation(ContinuationData::new_empty());
    let empty_cell = StackItem::Cell(SliceData::new_empty().cell());

    let mut info = SmartContractInfo::default();
    info.set_myself(MsgAddressInt::with_standart(None, 0, AccountId::from([0u8; 32])).unwrap());
    info.set_balance_remaining(CurrencyCollection::with_grams(10000));
    let mut c5_builder = BuilderData::new();
    c5_builder.append_reference(info.write_to_new_cell().unwrap());

    ctrls.put(0, &mut empty_cont.clone()).unwrap();
    ctrls.put(1, &mut empty_cont.clone()).unwrap();
    ctrls.put(3, &mut StackItem::Continuation(ContinuationData::with_code(code))).unwrap();
    ctrls.put(4, &mut StackItem::Cell(data.into_cell())).unwrap();
    ctrls.put(5, &mut StackItem::Cell(c5_builder.into())).unwrap();
    ctrls.put(6, &mut empty_cell.clone()).unwrap();
    ctrls
}

fn init_logger(debug: bool) {
    SimpleLogger::init(
        if debug {LevelFilter::Debug } else { LevelFilter::Info }, 
        Config { time: None, level: None, target: None, location: None, time_format: None },
    ).unwrap();
}

pub fn perform_contract_call(prog: &Program, body: Option<Arc<CellData>>, key_file: Option<&str>, debug: bool) {
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

    init_logger(debug);
    
    let code = prog.compile_asm().unwrap();
    let data = prog.data().unwrap();
    let registers = initialize_registers(code.clone(), data.into());
    stack
        .push(int!(0))
        .push(int!(0))
        .push(msg_cell)
        .push(StackItem::Slice(body)) 
        .push(int!(-1));

    let mut engine = Engine::new().setup(code, registers, stack)
        .unwrap_or_else(|e| panic!("Cannot setup engine, error {}", e));
    if debug { 
        engine.set_trace(Engine::TRACE_CODE);
    }
    let exit_code = match engine.execute() {
        Some(exc) => {
            println!("Unhandled exception: {}", exc); 
            exc.number
        },
        None => 0,
    };
    println!("TVM terminated with exit code {}", exit_code);
    engine.print_info_stack("Post-execution stack state");
}