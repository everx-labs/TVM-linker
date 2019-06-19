use keyman::KeypairManager;
use log::Level::Error;
use program::{load_from_file, save_to_file};
use simplelog::{SimpleLogger, Config, LevelFilter};
use sha2::Sha512;
use std::fmt;
use std::sync::Arc;
use std::time::SystemTime;
use tvm::cells_serialization::BagOfCells;
use tvm::executor::Engine;
use tvm::executor::gas::gas_state::Gas;
use tvm::stack::*;
use tvm::types::AccountId;
use tvm::bitstring::Bitstring;
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
    hdr.src = MsgAddressExt::with_extern(&Bitstring::create(vec![0x55; 8], 64)).unwrap();
    hdr.import_fee = Grams(0x1234u32.into());
    let mut msg = Message::with_ext_in_header(hdr);
    msg.body = body;
    msg
}

fn create_internal_msg(src_addr: AccountId, dst_addr: AccountId, value: u64, lt: u64, at: u32, body: Option<Arc<CellData>>) -> Message {
    let mut hdr = InternalMessageHeader::with_addresses(
        MsgAddressInt::with_standart(None, 0, src_addr).unwrap(),
        MsgAddressInt::with_standart(None, 0, dst_addr).unwrap(),
        CurrencyCollection::with_grams(value),
    );
    hdr.bounce = true;
    hdr.ihr_disabled = true;
    hdr.ihr_fee = Grams::from(0u64);
    hdr.created_lt = lt;
    hdr.created_at = UnixTime32(at);
    let mut msg = Message::with_int_header(hdr);
    msg.body = body;
    msg
}

fn sign_body(body: &mut SliceData, key_file: &str) {
    let pair = KeypairManager::from_secret_file(key_file).drain();
    let pub_key = pair.public.to_bytes();
    let signature = 
        pair.sign::<Sha512>(
            BagOfCells::with_root(body.clone()).get_repr_hash_by_index(0).unwrap().as_slice()
        ).to_bytes();
    let mut sign_builder = BuilderData::new();
    sign_builder.append_raw(&signature, signature.len() * 8).unwrap();
    sign_builder.append_raw(&pub_key, pub_key.len() * 8).unwrap();

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
        if debug {LevelFilter::Trace } else { LevelFilter::Info }, 
        Config { time: None, level: None, target: None, location: None, time_format: None },
    ).unwrap();
}



pub fn perform_contract_call(
    contract_file: &str, 
    body: Option<Arc<CellData>>, 
    key_file: Option<&str>, 
    debug: bool, 
    decode_actions: bool,
    msg_value: Option<&str>,
) -> i32 {
    let mut state_init = load_from_file(&format!("{}.tvc", contract_file));
    let contract_addr = AccountId::from(&[0x11u8; 32]);
    
    let mut stack = Stack::new();
    let is_internal;
    let value = if msg_value.is_some() {
        is_internal = true;
        u64::from_str_radix(msg_value.unwrap(), 10).unwrap()
    } else {
        is_internal = false;
        0
    };
    let msg = 
        if is_internal {
            create_internal_msg(
                AccountId::from(&[0u8; 32]), 
                contract_addr, 
                value,
                1,
                SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32,
                body.clone()
            )
        } else {
            create_external_inbound_msg(
                &contract_addr, 
                body.clone(),
            )
        };
    
    let msg_cell = StackItem::Cell(msg.write_to_new_cell().unwrap().into());

    let mut body: SliceData = match body {
        Some(b) => b.into(),
        None => BuilderData::new().into(),
    };

    if key_file.is_some() {
        sign_body(&mut body, key_file.unwrap());
    }

    if !log_enabled!(Error) {
        init_logger(debug);
    }

    let code: SliceData = state_init.code
            .clone()
            .unwrap_or(BuilderData::new().into())
            .into();
    let data = state_init.data
            .clone()
            .unwrap_or(BuilderData::new().into())
            .into();

    let registers = initialize_registers(code.clone(), data);
    let func_selector = if is_internal { 0 } else { -1 };
    stack
        .push(int!(100_000_000_000u64)) //contract balance: 100 grams
        .push(int!(value))           //msg balance
        .push(msg_cell)
        .push(StackItem::Slice(body)) 
        .push(int!(func_selector));

    let mut engine = Engine::new().setup(code, registers, stack, Gas::test())
        .unwrap_or_else(|e| panic!("Cannot setup engine, error {}", e));
    if debug { 
        engine.set_trace(Engine::TRACE_CODE);
    }
    let exit_code: i32 = match engine.execute() {
        Some(exc) => {
            println!("Unhandled exception: {}", exc); 
            exc.number as i32
        },
        None => 0i32,
    };
    println!("TVM terminated with exit code {}", exit_code);
    engine.print_info_stack("Post-execution stack state");
    engine.print_info_ctrls();

    if exit_code == 0 || exit_code == 1 {
            match engine.get_root() {
            StackItem::Cell(root_cell) => state_init.data = Some(root_cell),
            _ => panic!("cannot get root data: c4 register is not a cell."),
        };
        save_to_file(state_init, Some(contract_file)).expect("error");
        println!("Contract persistent data updated");
    }
    
    
    if decode_actions {
        if let StackItem::Cell(cell) = engine.get_actions() {
            let actions: OutActions = OutActions::construct_from(&mut cell.into()).expect("Failed to decode output actions");
            println!("Output actions:\n----------------");
            for act in actions {
                match act {
                    OutAction::SendMsg{mode: _, out_msg } => {
                        println!("Action(SendMsg):\n{}", MsgPrinter{ msg: out_msg });
                    },
                    _ => (),
                }
            }
        }
    }
    exit_code
}

struct MsgPrinter {
    pub msg: Arc<Message>,
}

impl fmt::Display for MsgPrinter {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "message header\n{}init  : {:?}\nbody  : {:?}\n",
            print_msg_header(&self.msg.header),
            self.msg.init,
            self.msg.body,
        )
    }    
}

fn print_msg_header(header: &CommonMsgInfo) -> String {
    match header {
        CommonMsgInfo::IntMsgInfo(header) => {
            format!("   ihr_disabled: {}\n", header.ihr_disabled) +
            &format!("   bounce      : {}\n", header.bounce) +
            &format!("   bounced     : {}\n", header.bounced) +
            &format!("   source      : {}\n", print_int_address(&header.src)) +
            &format!("   destination : {}\n", print_int_address(&header.dst)) +
            &format!("   value       : {}\n", header.value) +
            &format!("   ihr_fee     : {}\n", header.ihr_fee) +
            &format!("   fwd_fee     : {}\n", header.fwd_fee) +
            &format!("   created_lt  : {}\n", header.created_lt) +
            &format!("   created_at  : {}\n", header.created_at)
        },
        CommonMsgInfo::ExtInMsgInfo(header) => {
            format!("   source      : {}\n", print_ext_address(&header.src)) +
            &format!("   destination : {}\n", print_int_address(&header.dst)) +
            &format!("   import_fee  : {}\n", header.import_fee)
        },
        CommonMsgInfo::ExtOutMsgInfo(header) => {
            format!("   source      : {}\n", print_int_address(&header.src)) +
            &format!("   destination : {}\n", print_ext_address(&header.dst)) +
            &format!("   created_lt  : {}\n", header.created_lt) +
            &format!("   created_at  : {}\n", header.created_at)
        }
    }
}

fn print_int_address(addr: &MsgAddressInt) -> String {
    match addr {
        MsgAddressInt::AddrStd(ref std) => format!("{}:{:X}", std.workchain_id, std.address),
        MsgAddressInt::AddrVar(ref var) => format!("{}:{}", var.workchain_id, print_bitstring(&var.address)),
    }
}

fn print_bitstring(bits: &Bitstring) -> String {
    let mut res = String::new();
    for byte in bits.data() {
        res = res + &format!("{:02X}", byte);
    }
    res
}
fn print_ext_address(addr: &MsgAddressExt) -> String {
    match addr {
        MsgAddressExt::AddrNone => "AddrNone".to_string(),
        MsgAddressExt::AddrExtern(x) => format!("{}", x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msg_print() {
        let msg = create_external_inbound_msg(
            &AccountId::from([0x11; 32]), 
            Some(create_inbound_body(10, 20, 0x11223344)),
        );

        let msg2 = create_internal_msg(
            AccountId::from([0x11; 32]),
            AccountId::from([0x22; 32]),
            12345678,
            1,
            2,
            None,
        );

        println!("SendMsg action:\n{}", MsgPrinter{ msg: Arc::new(msg) });
        println!("SendMsg action:\n{}", MsgPrinter{ msg: Arc::new(msg2) });
    }

}
