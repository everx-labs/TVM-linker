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

use ed25519::signature::Signer;
use keyman::KeypairManager;
use log::Level::Error;
use crate::printer::msg_printer;
use program::{load_from_file, save_to_file, get_now};
use simplelog::{SimpleLogger, Config, LevelFilter};
use serde_json::Value;
use std::fs::File;
use std::str::FromStr;
use std::sync::Arc;
use ton_vm::executor::{Engine, EngineTraceInfo, EngineTraceInfoType, gas::gas_state::Gas};
use ton_vm::error::tvm_exception;
use ton_vm::stack::{StackItem, Stack, savelist::SaveList, integer::IntegerData};
use ton_vm::SmartContractInfo;
use ton_types::{AccountId, BuilderData, Cell, SliceData};
use ton_block::{
    CurrencyCollection, Deserializable, ExternalInboundMessageHeader, Grams,
    InternalMessageHeader, Message, MsgAddressExt, MsgAddressInt, OutAction,
    OutActions, Serializable, StateInit, UnixTime32
};
use ton_labs_assembler::DbgInfo;

const DEFAULT_ACCOUNT_BALANCE: &str = "100000000000";

#[allow(dead_code)]
fn create_inbound_body(a: i32, b: i32, func_id: i32) -> Result<Cell, String> {
    let mut builder = BuilderData::new();
    let version: u8 = 0;
    version.write_to(&mut builder)
        .map_err(|e| format!("Failed to write data: {}", e))?;
    func_id.write_to(&mut builder)
        .map_err(|e| format!("Failed to write data: {}", e))?;
    a.write_to(&mut builder)
        .map_err(|e| format!("Failed to write data: {}", e))?;
    b.write_to(&mut builder)
        .map_err(|e| format!("Failed to write data: {}", e))?;
    Ok(builder.into_cell()
        .map_err(|e| format!("Failed to convert builder to cell: {}", e))?)
}

fn create_external_inbound_msg(src_addr: MsgAddressExt, dst_addr: MsgAddressInt, body: Option<SliceData>) -> Message {
    let mut hdr = ExternalInboundMessageHeader::default();
    hdr.dst = dst_addr;
    hdr.src = src_addr;
    hdr.import_fee = Grams(0x1234u32.into());
    let mut msg = Message::with_ext_in_header(hdr);
    *msg.body_mut() = body;
    msg
}

fn create_internal_msg(
    src_addr: MsgAddressInt,
    dst_addr: MsgAddressInt,
    value: CurrencyCollection,
    lt: u64,
    at: u32,
    body: Option<SliceData>,
    bounced: bool,
) -> Message {
    let mut hdr = InternalMessageHeader::with_addresses(
        src_addr,
        dst_addr,
        value,
    );
    hdr.bounce = !bounced;
    hdr.bounced = bounced;
    hdr.ihr_disabled = true;
    hdr.ihr_fee = Grams::from(0u64);
    hdr.created_lt = lt;
    hdr.created_at = UnixTime32(at);
    let mut msg = Message::with_int_header(hdr);
    *msg.body_mut() = body;
    msg
}

fn sign_body(body: &mut SliceData, key_file: Option<&str>) -> Result<(), String>{
    let mut signed_body = BuilderData::from_slice(body);
    let mut sign_builder = BuilderData::new();
    if let Some(f) = key_file {
        let pair = KeypairManager::from_secret_file(f)
            .ok_or("Failed to read keypair.")?.drain();
        let pub_key = pair.public.to_bytes();
        let signature = pair.sign(body.cell().repr_hash().as_slice()).to_bytes();
        sign_builder.append_raw(&signature, signature.len() * 8)
            .map_err(|e| format!("Failed to write data: {}", e))?;
        sign_builder.append_raw(&pub_key, pub_key.len() * 8)
            .map_err(|e| format!("Failed to write data: {}", e))?;
    }
    signed_body.prepend_reference(sign_builder);
    *body = signed_body.into_cell()
        .map_err(|e| format!("Failed to convert builder to cell: {}", e))?.into();
    Ok(())
}

fn initialize_registers(data: SliceData, code: Cell, myself: MsgAddressInt, now: u32, balance: (u64, CurrencyCollection), config: Option<Cell>) -> Result<SaveList, String> {
    let mut ctrls = SaveList::new();
    let mut info = SmartContractInfo::with_myself(myself.serialize()
              .map_err(|e| format!("Failed to serialize address: {}", e))?.into());
    *info.balance_remaining_grams_mut() = balance.0 as u128;
    *info.balance_remaining_other_mut() = balance.1.other_as_hashmap().clone();
    *info.unix_time_mut() = now;
    if let Some(cell) = config {
        info.set_config_params(cell);
    }
    // TODO info.set_init_code_hash()
    info.set_mycode(code);
    ctrls.put(4, &mut StackItem::Cell(data.into_cell()))
        .map_err(|e| format!("Failed to convert data: {}", e))?;
    ctrls.put(7, &mut info.into_temp_data())
        .map_err(|e| format!("Failed to convert data: {}", e))?;
    Ok(ctrls)
}

fn init_logger(debug: bool) -> Result<(), String>{
    SimpleLogger::init(
        if debug {LevelFilter::Trace } else { LevelFilter::Info },
        Config { time: None, level: None, target: None, location: None, time_format: None },
    ).map_err(|e| format!("Failed to init logger: {}", e))?;
    Ok(())
    // TODO: it crashes sometimes here...
}


fn create_inbound_msg(
    selector: i32,
    msg_info: &MsgInfo,
    dst: AccountId,
) -> Result<Option<Message>, String> {
    let (_, value) = decode_balance(msg_info.balance)
        .map_err(|e| format!("Failed to decode balance: {}", e))?;
    Ok(match selector {
        0 => {
            let src = match msg_info.src {
                Some(s) => MsgAddressInt::from_str(s)
                    .map_err(|e| format!("Failed to convert address: {}", e))?,
                None => MsgAddressInt::with_standart(None, 0, [0u8; 32].into())
                    .map_err(|e| format!("Failed to convert address: {}", e))?,
            };
            Some(create_internal_msg(
                src,
                MsgAddressInt::with_standart(None, 0, dst)
                    .map_err(|e| format!("Failed to convert address: {}", e))?,
                value,
                1,
                get_now(),
                msg_info.body.clone(),
                msg_info.bounced,
            ))
        },
        -1 => {
            let src = match msg_info.src {
                Some(s) => MsgAddressExt::from_str(s)
                    .map_err(|e| format!("Failed to convert address: {}", e))?,
                None => {
                    MsgAddressExt::with_extern(
                        BuilderData::with_raw(vec![0x55; 8], 64)
                            .map_err(|e| format!("Failed to create builder: {}", e))?.into_cell()
                            .map_err(|e| format!("Failed to convert builder to cell: {}", e))?.into()
                    ).map_err(|e| format!("Failed to create address: {}", e))?
                },
            };
            Some(create_external_inbound_msg(
                src,
                MsgAddressInt::with_standart(None, 0, dst.clone())
                    .map_err(|e| format!("Failed to convert address: {}", e))?,
                msg_info.body.clone(),
            ))
        },
        _ => None,
    })
}

fn decode_actions<F>(actions: StackItem, state: &mut StateInit, action_decoder: F) -> Result<(), String>
    where F: Fn(SliceData, bool) -> ()
{
    if let StackItem::Cell(cell) = actions {
        let actions: OutActions = OutActions::construct_from(&mut cell.into())
            .map_err(|e| format!("Failed to decode output actions: {}", e))?;
        println!("Output actions:\n----------------");
        for act in actions {
            match act {
                OutAction::SendMsg{mode: _, out_msg } => {
                    println!("Action(SendMsg):\n{}", msg_printer(&out_msg)?);
                    if let Some(b) = out_msg.body() {
                        action_decoder(b, out_msg.is_internal());
                    }
                },
                OutAction::SetCode{ new_code: code } => {
                    println!("Action(SetCode)");
                    state.code = Some(code);
                },
                OutAction::ReserveCurrency { .. } => {
                    println!("Action(ReserveCurrency)");
                },
                OutAction::ChangeLibrary { .. } => {
                    println!("Action(ChangeLibrary)");
                },
                _ => println!("Action(Unknown)"),
            };
        }
    }
    Ok(())
}

fn load_code_and_data(state_init: &StateInit) -> (SliceData, SliceData) {
    let code: SliceData = state_init.code
            .clone()
            .unwrap_or(Cell::default())
            .into();
    let data = state_init.data
            .clone()
            .unwrap_or(Cell::default())
            .into();
    (code, data)
}


fn decode_balance(value: Option<&str>) -> Result<(u64, CurrencyCollection), String> {
    let value = value.unwrap_or(DEFAULT_ACCOUNT_BALANCE);
    if let Ok(main) = u64::from_str_radix(value, 10) {
        Ok((main, CurrencyCollection::with_grams(main)))
    } else {
        let err_msg = "invalid extra currencies";
        let v: Value = serde_json::from_str(value).map_err(|_| err_msg.to_owned())?;

        let main = v.get("main").and_then(|main| { main.as_u64() })
            .ok_or("invalid main currency".to_owned())?;

        let mut currencies = CurrencyCollection::with_grams(main);

        v.get("extra").and_then(|extra| {
            extra.as_object().and_then(|extra| {
                for (i, val) in extra {
                    let key = u32::from_str_radix(i, 10).ok()?;
                    let amount = val.as_u64()?;
                    currencies.set_other(key, amount as u128)
                        .map_err(|e| println!("Failed to update currencies: {}", e)).unwrap_or_default();
                }
                Some(())
            })
        }).ok_or(err_msg.to_owned())?;
        Ok((main, currencies))
    }
}

pub struct MsgInfo<'a> {
    pub balance: Option<&'a str>,
    pub src: Option<&'a str>,
    pub now: u32,
    pub bounced: bool,
    pub body: Option<SliceData>,
}

pub fn load_debug_info(
    filename: String,
) -> Option<DbgInfo> {
    match File::open(filename) {
        Ok(file) => {
            match serde_json::from_reader(file) {
                Ok(data) => Some(data),
                Err(_) => None
            }
        },
        Err(_) => None
    }
}

#[derive(PartialEq)]
pub enum TraceLevel {
    Full,
    Minimal,
    None
}

pub fn call_contract<F>(
    smc_file: &str,
    address: &str,
    smc_balance: Option<&str>,
    msg_info: MsgInfo,
    config_file: Option<&str>,
    key_file: Option<Option<&str>>,
    ticktock: Option<i8>,
    gas_limit: Option<i64>,
    action_decoder: Option<F>,
    trace_level: TraceLevel,
    debug_map_filename: String,
) -> Result<i32, String>
    where F: Fn(SliceData, bool)
{
    let wc = match msg_info.balance {
        Some(_) => 0,
        None => if ticktock.is_some() { -1 } else { 0 },
    };

    let addr = if address.find(':').is_none() {
        format!("{}:{}", wc, address)
    } else {
        address.to_owned()
    };
    let addr = ton_block::MsgAddressInt::from_str(&addr)
        .map_err(|e| format!("failed to load address: {}", e))?;

    let state_init = load_from_file(smc_file)?;
    let debug_info = load_debug_info(debug_map_filename);
    let config_cell = config_file.map(|filename| {
        let state = load_from_file(filename).unwrap_or_default();
        let (_code, data) = load_code_and_data(&state);
        // config dictionary is located in the first reference of the storage root cell
        data.into_cell().reference(0)
            .map_err(|e| format!("Failed to obtain dicitonary: {}", e))
    }).transpose()?;
    let (exit_code, state_init, is_vm_success) = call_contract_ex(
        addr, state_init, debug_info, smc_balance,
        msg_info, config_cell, key_file, ticktock, gas_limit, action_decoder, trace_level)?;
    if is_vm_success {
        save_to_file(state_init, Some(&smc_file), 0)
            .map_err(|e| format!("Failed to save file: {}", e))?;
        println!("Contract persistent data updated");
    }
    Ok(exit_code)
}

fn get_position(info: &EngineTraceInfo, debug_info: &Option<DbgInfo>) -> Option<String> {
    if let Some(debug_info) = debug_info {
        let cell_hash = info.cmd_code.cell().repr_hash();
        let offset = info.cmd_code.pos();
        let position = match debug_info.get(&cell_hash) {
            Some(offset_map) => match offset_map.get(&offset) {
                Some(pos) => format!("{}:{}", pos.filename, pos.line),
                None => String::from("-:0 (offset not found)")
            },
            None => String::from("-:0 (cell hash not found)")
        };
        return Some(position)
    }
    None
}

fn trace_callback_minimal(_engine: &Engine, info: &EngineTraceInfo, debug_info: &Option<DbgInfo>) {
    print!("{} {} {} {}", info.step, info.gas_used, info.gas_cmd, info.cmd_str);
    let position =  get_position(info, debug_info);
    if position.is_some() {
        print!(" {}", position.unwrap());
    }
    println!();
}

fn trace_callback(_engine: &Engine, info: &EngineTraceInfo, extended: bool, debug_info: &Option<DbgInfo>) {

    if info.info_type == EngineTraceInfoType::Dump {
        println!("{}", info.cmd_str);
        return;
    }

    println!("{}: {}",
        info.step,
        info.cmd_str
    );
    if extended {
        println!("{} {}",
            info.cmd_code.remaining_bits(),
            info.cmd_code.to_hex_string()
        );
    }
    println!("\nGas: {} ({})",
        info.gas_used,
        info.gas_cmd
    );

    let position = get_position(info, debug_info);
    if position.is_some() {
        println!("Position: {}", position.unwrap());
    }

    println!("\n--- Stack trace ------------------------");
    for item in info.stack.iter() {
        println!("{}", item);
    }
    println!("----------------------------------------\n");
}

pub fn call_contract_ex<F>(
    addr: MsgAddressInt,
    state_init: StateInit,
    debug_info: Option<DbgInfo>,
    smc_balance: Option<&str>,
    msg_info: MsgInfo,
    config: Option<Cell>,
    key_file: Option<Option<&str>>,
    ticktock: Option<i8>,
    gas_limit: Option<i64>,
    action_decoder: Option<F>,
    trace_level: TraceLevel,
) -> Result<(i32, StateInit, bool), String>
    where F: Fn(SliceData, bool)
{
    let func_selector = match msg_info.balance {
        Some(_) => 0,
        None => if ticktock.is_some() { -2 } else { -1 },
    };

    let msg = create_inbound_msg(func_selector, &msg_info, addr.address().clone())?;

    if !log_enabled!(Error) {
        init_logger(trace_level == TraceLevel::Full)?;
    }

    let mut state_init = state_init;
    let (code, data) = load_code_and_data(&state_init);

    let (smc_value, smc_balance) = decode_balance(smc_balance)?;
    let registers = initialize_registers(
        data,
        code.clone().into_cell(),
        addr.clone(),
        msg_info.now,
        (smc_value.clone(), smc_balance),
        config,
    )?;

    let mut stack = Stack::new();
    if func_selector > -2 {
        let msg_cell = StackItem::Cell(msg.ok_or("Failed to create message".to_string())?
            .serialize()
            .map_err(|e| format!("Failed to serialize message: {}", e))?);

        let mut body: SliceData = match msg_info.body {
            Some(b) => b.into(),
            None => Cell::default().into(),
        };

        if func_selector == -1 {
            if key_file.is_some() {
                sign_body(&mut body, key_file.unwrap())?;
            }
        }

        let msg_value = if func_selector == 0 {
            decode_balance(msg_info.balance)?.0 // for internal message
        } else {
            0 // for external message
        };

        stack
            .push(int!(smc_value))        // contract balance
            .push(int!(msg_value))        // msg value
            .push(msg_cell)               // whole msg
            .push(StackItem::Slice(body)) // msg body
            .push(int!(func_selector));   //selector
    } else {
        let addr_val = addr.address().to_hex_string();
        let addr_int = IntegerData::from_str_radix(&addr_val, 16)
            .map_err(|e| format!("Failed to convert address: {}", e))?;
        stack
            .push(int!(smc_value))
            .push(StackItem::Integer(Arc::new(addr_int))) //contract address
            .push(int!(ticktock.unwrap())) //tick or tock
            .push(int!(func_selector));
    }

    let gas = if let Some(gas_limit) = gas_limit {
        let mut tmp_gas = Gas::test();
        tmp_gas.new_gas_limit(gas_limit);
        tmp_gas
    } else {
        Gas::test()
    };

    let mut engine = Engine::new().setup_with_libraries(code, Some(registers), Some(stack), Some(gas), vec![]);
    engine.set_trace(0);
    match trace_level {
        TraceLevel::Full => engine.set_trace_callback(move |engine, info| { trace_callback(engine, info, true, &debug_info); }),
        TraceLevel::Minimal => engine.set_trace_callback(move |engine, info| { trace_callback_minimal(engine, info, &debug_info); }),
        TraceLevel::None => {}
    }
    let exit_code = match engine.execute() {
        Err(exc) => match tvm_exception(exc) {
            Ok(exc) => {
                println!("Unhandled exception: {}", exc);
                exc.exception_or_custom_code()
            }
            _ => -1
        }
        Ok(code) => code,
    };

    let is_vm_success = engine.get_committed_state().is_committed();
    println!("TVM terminated with exit code {}", exit_code);
    println!("Computing phase is success: {}", is_vm_success);
    println!("Gas used: {}", engine.get_gas().get_gas_used());
    println!();
    println!("{}", engine.dump_stack("Post-execution stack state", false));
    println!("{}", engine.dump_ctrls(false));

    if is_vm_success {
        if let Some(decoder) = action_decoder {
            decode_actions(engine.get_actions(), &mut state_init, decoder)?;
        }

        state_init.data = match engine.get_committed_state().get_root() {
            StackItem::Cell(root_cell) => Some(root_cell),
            _ => panic!("cannot get root data: c4 register is not a cell."),
        };
    }

    Ok((exit_code, state_init, is_vm_success))
}

#[cfg(test)]
pub fn perform_contract_call<F>(
    contract_file: &str,
    body: Option<SliceData>,
    key_file: Option<Option<&str>>,
    trace_level: TraceLevel,
    decode_c5: bool,
    msg_balance: Option<&str>,
    ticktock: Option<i8>,
    src: Option<&str>,
    balance: Option<&str>,
    now: u32,
    action_decoder: F,
) -> i32
    where F: Fn(SliceData, bool)
{
    let file = format!("{}.tvc", contract_file);
    call_contract(
        &file,
        contract_file,
        balance,
        MsgInfo{
            balance: msg_balance,
            src,
            now,
            bounced: false,
            body
        },
        None,
        key_file,
        ticktock,
        None,
        if decode_c5 { Some(action_decoder) } else { None },
        trace_level,
        String::from(""),
    ).unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msg_print() {
        let msg = create_external_inbound_msg(
            MsgAddressExt::with_extern(
                BuilderData::with_raw(vec![0x55; 8], 64).unwrap().into_cell().unwrap().into()
            ).unwrap(),
            MsgAddressInt::with_standart(None, 0, [0x11; 32].into()).unwrap(),
            Some(create_inbound_body(10, 20, 0x11223344).unwrap().into()),
        );

        let _msg2 = create_internal_msg(
            MsgAddressInt::with_standart(None, 0, [0x11; 32].into()).unwrap(),
            MsgAddressInt::with_standart(None, 0, [0x22; 32].into()).unwrap(),
            CurrencyCollection::with_grams(12345678),
            1,
            2,
            None,
            false,
        );

        println!("SendMsg action:\n{}", msg_printer(&msg).unwrap_or("Undefined".to_string()));
        println!("SendMsg action:\n{}", msg_printer(&msg).unwrap_or("Undefined".to_string()));
    }

    #[test]
    fn test_decode_balance() {
        let (main, balance) = decode_balance(Some(r#"{ "main": 100, "extra": {"0": 33, "50": 99} }"#)).unwrap();
        assert_eq!(main, 100);
        let mut expected_balance = CurrencyCollection::with_grams(100);
        expected_balance.set_other(0, 33).unwrap();
        expected_balance.set_other(50, 99).unwrap();
        assert_eq!(balance, expected_balance);

        let (main, balance) = decode_balance(Some("101")).unwrap();
        assert_eq!(main, 101);
        assert_eq!(balance, CurrencyCollection::with_grams(101));
    }

    #[test]
    fn test_decode_balance_default() {
        let (main, balance) = decode_balance(None).unwrap();
        let expected = u64::from_str_radix(DEFAULT_ACCOUNT_BALANCE, 10).unwrap();
        assert_eq!(main, expected);
        assert_eq!(balance, CurrencyCollection::with_grams(expected));
    }

    #[test]
    fn test_decode_balance_invalid() {
        let err = decode_balance(Some(r#"{ "main": 100 }"#));
        assert_eq!(err.is_err(), true);

        let err = decode_balance(Some(r#"{ "main": qwe }"#));
        assert_eq!(err.is_err(), true);

        let err = decode_balance(Some(r#"{ "main": 0, extra: {"dd": 10} }"#));
        assert_eq!(err.is_err(), true);

        let err = decode_balance(Some(r#"{ "main": 0, extra: {"0": qwe} }"#));
        assert_eq!(err.is_err(), true);
    }
}
