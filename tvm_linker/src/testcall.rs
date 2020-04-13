/*
 * Copyright 2018-2019 TON DEV SOLUTIONS LTD.
 *
 * Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
 * this file except in compliance with the License.  You may obtain a copy of the
 * License at: https://ton.dev/licenses
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific TON DEV software governing permissions and
 * limitations under the License.
 */
use keyman::KeypairManager;
use log::Level::Error;
use printer::MsgPrinter;
use program::{load_from_file, save_to_file};
use simplelog::{SimpleLogger, Config, LevelFilter};
use serde_json::Value;
use sha2::Sha512;
use std::str::FromStr;
use std::sync::Arc;
use std::time::SystemTime;
use ton_vm::executor::{Engine, gas::gas_state::Gas};
use ton_vm::error::TvmError;
use ton_vm::stack::{StackItem, Stack, savelist::SaveList, integer::IntegerData};
use ton_vm::SmartContractInfo;
use ton_types::{AccountId, BuilderData, Cell, SliceData};
use ton_block::{
    CurrencyCollection, Deserializable, ExternalInboundMessageHeader, Grams, 
    InternalMessageHeader, Message, MsgAddressExt, MsgAddressInt, OutAction, 
    OutActions, Serializable, StateInit, UnixTime32
};

#[allow(dead_code)]
fn create_inbound_body(a: i32, b: i32, func_id: i32) -> Cell {
    let mut builder = BuilderData::new();
    let version: u8 = 0;
    version.write_to(&mut builder).unwrap();
    func_id.write_to(&mut builder).unwrap();
    a.write_to(&mut builder).unwrap();
    b.write_to(&mut builder).unwrap();
    builder.into()
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
) -> Message {
    let mut hdr = InternalMessageHeader::with_addresses(
        src_addr,
        dst_addr,
        value,
    );
    hdr.bounce = true;
    hdr.ihr_disabled = true;
    hdr.ihr_fee = Grams::from(0u64);
    hdr.created_lt = lt;
    hdr.created_at = UnixTime32(at);
    let mut msg = Message::with_int_header(hdr);
    *msg.body_mut() = body;
    msg
}

fn sign_body(body: &mut SliceData, key_file: Option<&str>) {
    let mut signed_body = BuilderData::from_slice(body);
    let mut sign_builder = BuilderData::new();
    if let Some(f) = key_file {
        let pair = KeypairManager::from_secret_file(f).drain();
        let pub_key = pair.public.to_bytes();
        let signature = pair.sign::<Sha512>(body.cell().repr_hash().as_slice()).to_bytes();
        sign_builder.append_raw(&signature, signature.len() * 8).unwrap();
        sign_builder.append_raw(&pub_key, pub_key.len() * 8).unwrap();
    }
    signed_body.prepend_reference(sign_builder);
    *body = signed_body.into();
}

fn initialize_registers(data: SliceData, myself: MsgAddressInt, now: u32, balance: (u64, CurrencyCollection)) -> SaveList {
    let mut ctrls = SaveList::new();
    let mut info = SmartContractInfo::with_myself(myself.write_to_new_cell().unwrap().into());
    *info.balance_remaining_grams_mut() = balance.0 as u128;
    *info.balance_remaining_other_mut() = balance.1.other;
    *info.unix_time_mut() = now;
    ctrls.put(4, &mut StackItem::Cell(data.into_cell())).unwrap();
    ctrls.put(7, &mut info.into_temp_data()).unwrap();
    ctrls
}

fn init_logger(debug: bool) {
    SimpleLogger::init(
        if debug {LevelFilter::Trace } else { LevelFilter::Info }, 
        Config { time: None, level: None, target: None, location: None, time_format: None },
    ).unwrap();
}


fn create_inbound_msg(
    selector: i32,
    src: Option<&str>,
    dst: AccountId,
    value: CurrencyCollection,
    body: Option<SliceData>,
) -> Option<Message> {
    match selector {
        0 => {
            let src = match src {
                Some(s) => MsgAddressInt::from_str(s).unwrap(),
                None => MsgAddressInt::with_standart(None, 0, [0u8; 32].into()).unwrap(),
            };
            Some(create_internal_msg(
                src,
                MsgAddressInt::with_standart(None, 0, dst).unwrap(),
                value,
                1,
                SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32,
                body,
            ))
        },
        -1 => {
            let src = match src {
                Some(s) => MsgAddressExt::from_str(s).unwrap(),
                None => {
                    MsgAddressExt::with_extern(
                        BuilderData::with_raw(vec![0x55; 8], 64).unwrap().into()
                    ).unwrap()
                },
            };
            Some(create_external_inbound_msg(
                src,
                MsgAddressInt::with_standart(None, 0, dst.clone()).unwrap(), 
                body,
            ))
        },
        _ => None,
    }
}

fn decode_actions<F>(actions: StackItem, state: &mut StateInit, action_decoder: F) 
    where F: Fn(SliceData, bool) -> () 
{
    if let StackItem::Cell(cell) = actions {
        let actions: OutActions = OutActions::construct_from(&mut cell.into())
            .expect("Failed to decode output actions");
        println!("Output actions:\n----------------");
        for act in actions {
            match act {
                OutAction::SendMsg{mode: _, out_msg } => {
                    println!("Action(SendMsg):\n{}", MsgPrinter{ msg: out_msg.clone() });
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
}

fn load_code_and_data(state_init: &StateInit) -> (SliceData, SliceData) {
    let code: SliceData = state_init.code
            .clone()
            .unwrap_or(BuilderData::new().into())
            .into();
    let data = state_init.data
            .clone()
            .unwrap_or(BuilderData::new().into())
            .into();
    (code, data)
}


fn decode_balance(value: Option<&str>) -> Result<(u64, CurrencyCollection), String> {
    let value = value.unwrap_or("0");
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
                    currencies.set_other(key, amount as u128);
                }
                Some(())
            })
        }).ok_or(err_msg.to_owned())?;

        Ok((main, currencies))
    }
}

pub fn perform_contract_call<F>(
    contract_file: &str, 
    body: Option<SliceData>, 
    key_file: Option<Option<&str>>, 
    debug: bool, 
    decode_c5: bool,
    msg_value: Option<&str>,
    ticktock: Option<&str>,
    src_str: Option<&str>,
    balance: Option<&str>,
    now: u32,
    action_decoder: F,
) -> i32
    where F: Fn(SliceData, bool)
{
    let addr = AccountId::from_str(contract_file).unwrap();
    
    let func_selector = match msg_value {
        Some(_) => 0,    
        None => if ticktock.is_some() { -2 } else { -1 },
    };
    
    let (value, msg_balance) = decode_balance(msg_value).unwrap();
    
    let msg = create_inbound_msg(func_selector, src_str, addr.clone(), msg_balance, body.clone());
        
    if !log_enabled!(Error) {
        init_logger(debug);
    }
    
    let mut state_init = load_from_file(&format!("{}.tvc", contract_file));
    let (code, data) = load_code_and_data(&state_init);
    
    let workchain_id = if func_selector > -2 { 0 } else { -1 };
    let (acc_value, account_balance) = decode_balance(balance).unwrap();
    let registers = initialize_registers(
        data,
        MsgAddressInt::with_standart(None, workchain_id, addr.clone()).unwrap(),
        now,
        (acc_value.clone(), account_balance),
    );

    let mut stack = Stack::new();
    if func_selector > -2 {
        let msg_cell = StackItem::Cell(msg.unwrap().write_to_new_cell().unwrap().into());

        let mut body: SliceData = match body {
            Some(b) => b.into(),
            None => BuilderData::new().into(),
        };
        
        if func_selector == -1 {
            key_file.map(|key| sign_body(&mut body, key));
        }

        stack
            .push(int!(acc_value))
            .push(int!(value))              //msg balance
            .push(msg_cell)                 //msg
            .push(StackItem::Slice(body))   //msg.body
            .push(int!(func_selector));     //selector
    } else {
        stack
            .push(int!(acc_value))
            .push(StackItem::Integer(Arc::new(IntegerData::from_str_radix(contract_file, 16).unwrap()))) //contract address
            .push(int!(i8::from_str_radix(ticktock.unwrap(), 10).unwrap())) //tick or tock
            .push(int!(func_selector));
    }

    let mut engine = Engine::new().setup(code, Some(registers), Some(stack), Some(Gas::test()));
    engine.set_trace(if debug {Engine::TRACE_ALL} else {0});
    let exit_code: i32 = match engine.execute() {
        Ok(code) => {
            code as i32
        }
        Err(exc) => if let Ok(TvmError::TvmExceptionFull(exc)) = exc.downcast() {
            println!("Unhandled exception: {}", exc);
            exc.number as i32
        } else {
            -1
        }
    };
    println!("TVM terminated with exit code {}", exit_code);
    println!("Gas used: {}", engine.get_gas().get_gas_used());
    println!("");
    println!("{}", engine.dump_stack("Post-execution stack state", false));
    println!("{}", engine.dump_ctrls(false));
    
    if decode_c5 {
        decode_actions(engine.get_actions(), &mut state_init, action_decoder);
    }

    if exit_code == 0 || exit_code == 1 {
        state_init.data = match engine.get_committed_state().get_root() {
            StackItem::Cell(root_cell) => Some(root_cell),
            _ => panic!("cannot get root data: c4 register is not a cell."),
        };
        save_to_file(state_init, Some(contract_file), 0).expect("error");
        println!("Contract persistent data updated");
    }

    exit_code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msg_print() {
        let msg = create_external_inbound_msg(
            MsgAddressExt::with_extern(
                BuilderData::with_raw(vec![0x55; 8], 64).unwrap().into()
            ).unwrap(),
            MsgAddressInt::with_standart(None, 0, [0x11; 32].into()).unwrap(),
            Some(create_inbound_body(10, 20, 0x11223344).into()),
        );

        let msg2 = create_internal_msg(
            MsgAddressInt::with_standart(None, 0, [0x11; 32].into()).unwrap(),
            MsgAddressInt::with_standart(None, 0, [0x22; 32].into()).unwrap(),
            CurrencyCollection::with_grams(12345678),
            1,
            2,
            None,
        );

        println!("SendMsg action:\n{}", MsgPrinter{ msg: Arc::new(msg) });
        println!("SendMsg action:\n{}", MsgPrinter{ msg: Arc::new(msg2) });
    }

}