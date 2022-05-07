/*
 * Copyright 2018-2022 TON DEV SOLUTIONS LTD.
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
use failure::format_err;
use keyman::KeypairManager;
use log::Level::Error;
use crate::printer::msg_printer;
use program::{load_from_file, get_now};
use simplelog::{SimpleLogger, Config, LevelFilter};
use serde_json::Value;
use std::fs::File;
use std::str::FromStr;
use std::sync::Arc;
use ton_vm::executor::{Engine, EngineTraceInfo, EngineTraceInfoType, gas::gas_state::Gas};
use ton_vm::error::tvm_exception;
use ton_vm::stack::{StackItem, Stack, savelist::SaveList, integer::IntegerData};
use ton_vm::SmartContractInfo;
use ton_types::{AccountId, BuilderData, Cell, SliceData, Result, Status};
use ton_block::{
    CurrencyCollection, Deserializable, ExternalInboundMessageHeader, Grams,
    InternalMessageHeader, Message, MsgAddressExt, MsgAddressInt, OutAction,
    OutActions, Serializable, StateInit, UnixTime32
};
use ton_labs_assembler::DbgInfo;

const DEFAULT_ACCOUNT_BALANCE: &str = "100000000000";

fn create_external_inbound_msg(src: MsgAddressExt, dst: MsgAddressInt, body: Option<SliceData>) -> Message {
    let hdr = ExternalInboundMessageHeader {
        dst,
        src,
        import_fee: 0x1234u64.into()
    };
    let mut msg = Message::with_ext_in_header(hdr);
    if let Some(body) = body {
        msg.set_body(body);
    }
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
    hdr.created_at = UnixTime32::new(at);
    let mut msg = Message::with_int_header(hdr);
    if let Some(body) = body {
        msg.set_body(body);
    }
    msg
}

fn sign_body(body: &mut SliceData, key_file: Option<&str>) -> Status {
    let mut signed_body = BuilderData::from_slice(body);
    let mut sign_builder = BuilderData::new();
    if let Some(f) = key_file {
        let pair = KeypairManager::from_secret_file(f)
            .ok_or_else(|| format_err!("Failed to read keypair."))?.drain();
        let pub_key = pair.public.to_bytes();
        let signature = pair.sign(body.cell().repr_hash().as_slice()).to_bytes();
        sign_builder.append_raw(&signature, signature.len() * 8)?;
        sign_builder.append_raw(&pub_key, pub_key.len() * 8)?;
    }
    signed_body.prepend_reference(sign_builder);
    *body = signed_body.into_cell()?.into();
    Ok(())
}

fn initialize_registers(data: SliceData, code: Cell, myself: MsgAddressInt, now: u32, balance: (u64, CurrencyCollection), config: Option<Cell>) -> Result<SaveList> {
    let mut ctrls = SaveList::new();
    let mut info = SmartContractInfo::with_myself(myself.serialize()?.into());
    *info.balance_remaining_grams_mut() = balance.0 as u128;
    *info.balance_remaining_other_mut() = balance.1.other_as_hashmap();
    *info.unix_time_mut() = now;
    if let Some(cell) = config {
        info.set_config_params(cell);
    }
    // TODO info.set_init_code_hash()
    info.set_mycode(code);
    ctrls.put(4, &mut StackItem::Cell(data.into_cell()))?;
    ctrls.put(7, &mut info.into_temp_data())?;
    Ok(ctrls)
}

fn init_logger(debug: bool) -> Status {
    SimpleLogger::init(
        if debug {LevelFilter::Trace } else { LevelFilter::Info },
        Config { time: None, level: None, target: None, location: None, time_format: None },
    )?;
    Ok(())
}


fn create_inbound_msg(
    selector: i32,
    msg_info: &MsgInfo,
    dst: AccountId,
) -> Result<Option<Message>> {
    let (_, value) = decode_balance(msg_info.balance)?;
    Ok(match selector {
        0 => {
            let src = match msg_info.src {
                Some(s) => MsgAddressInt::from_str(s)?,
                None => MsgAddressInt::with_standart(None, 0, [0u8; 32].into())?,
            };
            Some(create_internal_msg(
                src,
                MsgAddressInt::with_standart(None, 0, dst)?,
                value,
                1,
                get_now(),
                msg_info.body.clone(),
                msg_info.bounced,
            ))
        },
        -1 => {
            let src = match msg_info.src {
                Some(s) => MsgAddressExt::from_str(s)?,
                None => {
                    MsgAddressExt::with_extern(
                        BuilderData::with_raw(vec![0x55; 8], 64)?.into_cell()?.into()
                    ).map_err(|e| format_err!("Failed to create address: {}", e))?
                },
            };
            Some(create_external_inbound_msg(
                src,
                MsgAddressInt::with_standart(None, 0, dst)
                    .map_err(|e| format_err!("Failed to convert address: {}", e))?,
                msg_info.body.clone(),
            ))
        },
        _ => None,
    })
}

fn decode_actions<F>(actions: StackItem, state: &mut StateInit, action_decoder: F) -> Status
    where F: Fn(SliceData, bool)
{
    if let StackItem::Cell(cell) = actions {
        let actions: OutActions = OutActions::construct_from(&mut cell.into())?;
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

pub fn load_code_and_data(state_init: &StateInit) -> (SliceData, SliceData) {
    let code: SliceData = state_init.code.clone().unwrap_or_default().into();
    let data = state_init.data.clone().unwrap_or_default().into();
    (code, data)
}

fn decode_balance(value: Option<&str>) -> Result<(u64, CurrencyCollection)> {
    let value = value.unwrap_or(DEFAULT_ACCOUNT_BALANCE);
    if let Ok(main) = value.parse::<u64>() {
        Ok((main, CurrencyCollection::with_grams(main)))
    } else {
        let err_msg = "invalid extra currencies";
        let v: Value = serde_json::from_str(value).map_err(|e| format_err!("{}: {}", err_msg, e))?;

        let main = v.get("main").and_then(|main| { main.as_u64() })
            .ok_or_else(|| format_err!("invalid main currency"))?;

        let mut currencies = CurrencyCollection::with_grams(main);

        v.get("extra").and_then(|extra| {
            extra.as_object().and_then(|extra| {
                for (i, val) in extra {
                    let key = i.parse::<u32>().ok()?;
                    let amount = val.as_u64()?;
                    currencies.set_other(key, amount as u128)
                        .map_err(|e| println!("Failed to update currencies: {}", e)).unwrap_or_default();
                }
                Some(())
            })
        }).ok_or_else(|| format_err!("{}", err_msg))?;
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

pub fn load_debug_info(filename: &str) -> Option<DbgInfo> {
    File::open(filename)
        .ok()
        .and_then(|file| { serde_json::from_reader(file).ok() })
        .flatten()
}

pub fn load_config(filename: &str) -> Option<Cell> {
    let state = load_from_file(filename).unwrap_or_default();
    let (_code, data) = load_code_and_data(&state);
    // config dictionary is located in the first reference of the storage root cell
    data.into_cell().reference(0).ok()
}

#[derive(PartialEq)]
pub enum TraceLevel {
    Full,
    Minimal,
    None
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
        return
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

pub struct TestCallParams<'a, F: Fn(SliceData, bool)> {
    pub balance: Option<&'a str>,
    pub msg_info: MsgInfo<'a>,
    pub config: Option<Cell>,
    pub key_file: Option<Option<&'a str>>,
    pub ticktock: Option<i8>,
    pub gas_limit: Option<i64>,
    pub action_decoder: Option<F>,
    pub trace_level: TraceLevel,
    pub debug_info: Option<DbgInfo>,
    pub capabilities: Option<u64>
}

pub fn call_contract<F>(
    addr: MsgAddressInt,
    state_init: StateInit,
    params: TestCallParams<F>,
) -> Result<(i32, StateInit, bool)>
    where F: Fn(SliceData, bool)
{
    let func_selector = match params.msg_info.balance {
        Some(_) => 0,
        None => if params.ticktock.is_some() { -2 } else { -1 },
    };

    let msg = create_inbound_msg(func_selector, &params.msg_info, addr.address())?;

    if !log_enabled!(Error) {
        init_logger(params.trace_level == TraceLevel::Full)?;
    }

    let mut state_init = state_init;
    let (code, data) = load_code_and_data(&state_init);

    let (smc_value, smc_balance) = decode_balance(params.balance)?;
    let registers = initialize_registers(
        data,
        code.clone().into_cell(),
        addr.clone(),
        params.msg_info.now,
        (smc_value, smc_balance),
        params.config,
    )?;

    let mut stack = Stack::new();
    if func_selector > -2 {
        let msg_cell = StackItem::Cell(
            msg.ok_or_else(|| format_err!("Failed to create message"))?.serialize()?
        );

        let mut body = match params.msg_info.body {
            Some(b) => b,
            None => Cell::default().into(),
        };

        if func_selector == -1 {
            if let Some(key_file) = params.key_file {
                sign_body(&mut body, key_file)?;
            }
        }

        let msg_value = if func_selector == 0 {
            decode_balance(params.msg_info.balance)?.0 // for internal message
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
        let addr_int = IntegerData::from_str_radix(&addr_val, 16)?;
        stack
            .push(int!(smc_value))
            .push(StackItem::Integer(Arc::new(addr_int))) //contract address
            .push(int!(params.ticktock.unwrap())) //tick or tock
            .push(int!(func_selector));
    }

    let gas = if let Some(gas_limit) = params.gas_limit {
        let mut tmp_gas = Gas::test();
        tmp_gas.new_gas_limit(gas_limit);
        tmp_gas
    } else {
        Gas::test()
    };

    let mut engine = Engine::with_capabilities(
        params.capabilities.unwrap_or(0)
    ).setup_with_libraries(
        code, Some(registers), Some(stack), Some(gas), vec![]
    );
    engine.set_trace(0);
    let debug_info = params.debug_info;
    match params.trace_level {
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
        if let Some(decoder) = params.action_decoder {
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
mod tests {
    use super::*;

    fn create_inbound_body(a: i32, b: i32, func_id: i32) -> Result<Cell> {
        let mut builder = BuilderData::new();
        let version: u8 = 0;
        version.write_to(&mut builder)?;
        func_id.write_to(&mut builder)?;
        a.write_to(&mut builder)?;
        b.write_to(&mut builder)?;
        builder.into_cell()
    }

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
