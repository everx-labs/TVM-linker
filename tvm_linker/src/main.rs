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
extern crate ton_abi as abi_json;
extern crate base64;
#[macro_use]
extern crate clap;
extern crate crc16;
extern crate ed25519;
extern crate ed25519_dalek;
#[macro_use]
extern crate lazy_static;
extern crate rand;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate sha2;
extern crate simplelog;
extern crate ton_block;
extern crate ton_types;
#[macro_use]
extern crate ton_vm;
#[macro_use]
extern crate log;
extern crate ton_sdk;
extern crate ton_labs_assembler;
extern crate num_traits;

mod abi;
mod initdata;
mod keyman;
mod parser;
mod printer;
mod program;
mod real_ton;
mod resolver;
mod methdict;
mod testcall;
mod disasm;

use abi::{build_abi_body, decode_body, load_abi_json_string, load_abi_contract};
use clap::ArgMatches;
use initdata::set_initial_data;
use keyman::KeypairManager;
use parser::{ParseEngine, ParseEngineResults};
use program::{Program, get_now};
use real_ton::{decode_boc, compile_message};
use resolver::resolve_name;
use ton_block::{Deserializable, Message};
use std::{path::Path};
use testcall::{call_contract, MsgInfo, TraceLevel};
use ton_types::{BuilderData, SliceData};
use std::env;
use disasm::disasm_command;
use ton_labs_assembler::Line;
use std::fs::File;

use crate::real_ton::load_stateinit;

fn main() -> Result<(), i32> {
    linker_main().map_err(|err_str| {
        println!("Error: {}", err_str);
        1
    })
}

fn linker_main() -> Result<(), String> {
    let build_info = format!(
        "v{}\nBUILD_GIT_COMMIT: {}\nBUILD_GIT_DATE:   {}\nBUILD_TIME:       {}",
        env!("CARGO_PKG_VERSION"),
        env!("BUILD_GIT_COMMIT"),
        env!("BUILD_GIT_DATE"),
        env!("BUILD_TIME") ,
    );
    let matches = clap_app!(tvm_linker =>
        (version: build_info.as_str())
        (author: "TON Labs")
        (about: "Tool for assembling, disassembling and executing TVM code")
        (@subcommand decode =>
            (about: "take apart a message boc or a tvc file")
            (version: build_info.as_str())
            (author: "TON Labs")
            (@arg INPUT: +required +takes_value "BOC file")
            (@arg TVC: --tvc "BOC file is tvc file")
        )
        (@subcommand compile =>
            (@setting AllowNegativeNumbers)
            (about: "compile contract")
            (version: build_info.as_str())
            (author: "TON Labs")
            (@arg INPUT: +required +takes_value "TVM assembler source file")
            (@arg ABI: -a --("abi-json") +takes_value "Supplies contract abi to calculate correct function ids. If not specified abi is loaded from file path obtained from <INPUT> path if it exists.")
            (@arg CTOR_PARAMS: -p --("ctor-params") +takes_value "Supplies arguments for the constructor")
            (@arg GENKEY: --genkey +takes_value conflicts_with[SETKEY] "Generates new keypair for the contract and saves it to the file")
            (@arg SETKEY: --setkey +takes_value conflicts_with[GENKEY] "Loads existing keypair from the file")
            (@arg WC: -w +takes_value "Workchain id used to print contract address, -1 by default.")
            (@arg DEBUG: --debug "Prints debug info: xref table and parsed assembler sources")
            (@arg DEBUG_MAP: --("debug-map") +takes_value "Generates debug map file")
            (@arg DATA: --("data") +takes_value "Overwrites data with a cell from a file")
            (@arg LIB: --lib +takes_value ... number_of_values(1) "Standard library source file. If not specified lib is loaded from environment variable TVM_LINKER_LIB_PATH if it exists.")
            (@arg OUT_FILE: -o +takes_value "Output file name")
            (@arg LANGUAGE: --language +takes_value "Enable language-specific features in linkage")
        )
        (@subcommand test =>
            (@setting AllowLeadingHyphen)
            (about: "execute contract in test environment")
            (version: build_info.as_str())
            (author: "TON Labs")
            (@arg SOURCE: -s --source +takes_value "Contract source file")
            (@arg BODY: --body +takes_value "Body for external inbound message (a bitstring like x09c_ or a hex string)")
            (@arg BODY_FROM_BOC: --("body-from-boc") +takes_value "Body from message boc file")
            (@arg SIGN: --sign +takes_value "Signs body with private key from defined file")
            (@arg TRACE: --trace "Prints last command name, stack and registers after each executed TVM command")
            (@arg TRACE_MIN: --("trace-minimal") "Prints minimal trace")
            (@arg DECODEC6: --("decode-c6") "Prints last command name, stack and registers after each executed TVM command")
            (@arg INTERNAL: --internal +takes_value "Emulates inbound internal message with value instead of external message")
            (@arg BOUNCED: --bounced requires[INTERNAL] "Emulates bounced message, can be used only with --internal option.")
            (@arg BALANCE: --balance +takes_value "Emulates supplied account balance")
            (@arg SRCADDR: --src +takes_value "Supplies message source address")
            (@arg NOW: --now +takes_value "Supplies transaction creation unixtime")
            (@arg TICKTOCK: --ticktock +takes_value conflicts_with[BODY] "Emulates ticktock transaction in masterchain, 0 for tick and -1 for tock")
            (@arg GASLIMIT: -l --("gas-limit") +takes_value "Defines gas limit for tvm execution")
            (@arg CONFIG: --config +takes_value "Imports config parameters from a config contract boc")
            (@arg INPUT: +required +takes_value "TVM assembler source file or contract name if used with test subcommand")
            (@arg ADDRESS: --address +takes_value "Contract address, placed on the stack. If not specified address can be obtained from the INPUT argument.")
            (@arg ABI_JSON: -a --("abi-json") +takes_value conflicts_with[BODY] "Supplies json file with contract ABI")
            (@arg ABI_METHOD: -m --("abi-method") +takes_value conflicts_with[BODY] "Supplies the name of the calling contract method")
            (@arg ABI_PARAMS: -p --("abi-params") +takes_value conflicts_with[BODY] "Supplies ABI arguments for the contract method")
            (@arg ABI_HEADER: -h --("abi-header") +takes_value conflicts_with[BODY] conflicts_with[INTERNAL] "Supplies ABI header")
        )
        (@subcommand message =>
            (@setting AllowNegativeNumbers)
            (about: "generate external inbound message for the blockchain")
            (version: build_info.as_str())
            (author: "TON Labs")
            (@arg INIT: -i --init "Generates constructor message with code and data of the contract")
            (@arg DATA: -d --data +takes_value "Supplies body for the message in hex format (empty data by default)")
            (@arg WORKCHAIN: -w --workchain +takes_value "Supplies workchain id for the contract address")
            (@arg ABI_JSON: -a --("abi-json") +takes_value conflicts_with[DATA] "Supplies json file with contract ABI")
            (@arg ABI_METHOD: -m --("abi-method") +takes_value conflicts_with[DATA] "Supplies the name of the calling contract method")
            (@arg ABI_PARAMS: -p --("abi-params") +takes_value conflicts_with[DATA] "Supplies ABI arguments for the contract method")
            (@arg ABI_HEADER: -h --("abi-header") +takes_value conflicts_with[DATA] "Supplies ABI header")
            (@arg SIGN: --setkey +takes_value "Loads existing keypair from the file")
            (@arg INPUT: +required +takes_value "TVM assembler source file or contract name")
        )
        (@subcommand init =>
            (about: "initialize smart contract public variables")
            (version: build_info.as_str())
            (@arg INPUT: +required +takes_value "Path to compiled smart contract file")
            (@arg DATA: +required +takes_value "Set of public variables with values in json format")
            (@arg ABI: +required +takes_value "Path to smart contract ABI file")
        )
        (@subcommand disasm =>
            (about: "disassemble a tvc or dumps its tree of cells")
            (version: build_info.as_str())
            (author: "TON Labs")
            (@subcommand dump =>
                (about: "dumps tree of cells for the given tvc")
                (version: build_info.as_str())
                (@arg TVC: +required +takes_value "Path to tvc file")
            )
            (@subcommand graphviz =>
                (about: "generates graphviz dot for the given tvc")
                (version: build_info.as_str())
                (@arg METHOD: --method +takes_value "Selects a particular method by ID or int|ext|ticktock")
                (@arg TVC: +required +takes_value "Path to tvc file")
            )
            (@subcommand solidity =>
                (about: "disassembles the given tvc produced by Solidity compiler")
                (version: build_info.as_str())
                (@arg TVC: +required +takes_value "Path to tvc file")
            )
            (@subcommand solidity_v1 =>
                (about: "disassembles the given tvc produced by Solidity compiler, obsolete version")
                (version: build_info.as_str())
                (@arg TVC: +required +takes_value "Path to tvc file")
            )
            (@subcommand fun_c =>
                (about: "disassembles the given tvc produced by FunC compiler")
                (version: build_info.as_str())
                (@arg TVC: +required +takes_value "Path to tvc file")
            )
        )
        (@setting SubcommandRequired)
    ).get_matches();

    //SUBCOMMAND INIT
    if let Some(matches) = matches.subcommand_matches("init") {
        return run_init_subcmd(matches);
    }

    //SUBCOMMAND TEST
    if let Some(test_matches) = matches.subcommand_matches("test") {
        return run_test_subcmd(test_matches);
    }

    //SUBCOMMAND DECODE
    if let Some(decode_matches) = matches.subcommand_matches("decode") {
        decode_boc(
            decode_matches.value_of("INPUT").unwrap(),
            decode_matches.is_present("TVC"),
        );
        return Ok(());
    }

    //SUBCOMMAND MESSAGE
    if let Some(msg_matches) = matches.subcommand_matches("message") {
        let mut suffix = String::new();
        suffix += "-msg";
        if msg_matches.is_present("INIT") {
            suffix += "-init";
        }
        if msg_matches.is_present("DATA") || msg_matches.is_present("ABI_JSON") {
            suffix += "-body";
        }
        suffix += ".boc";

        let msg_body = match msg_matches.value_of("DATA") {
            Some(data) => {
                let buf = hex::decode(data).map_err(|_| "data argument has invalid format".to_string())?;
                let len = buf.len() * 8;
                let body: SliceData = BuilderData::with_raw(buf, len)
                    .map_err(|e| format!("failed to pack body in cell: {}", e))?
                    .into_cell()
                    .map_err(|e| format!("failed to pack body in cell: {}", e))?
                    .into();
                Some(body)
            },
            None => {
                build_body(msg_matches)?
            },
        };

        return compile_message(
            msg_matches.value_of("INPUT").unwrap(),
            msg_matches.value_of("WORKCHAIN"),
            msg_body,
            msg_matches.is_present("INIT"),
            &suffix,
        )
    }

    //SUBCOMMAND COMPILE
    if let Some(compile_matches) = matches.subcommand_matches("compile") {
        let input = compile_matches.value_of("INPUT").unwrap();
        let abi_from_input = format!("{}{}", input.trim_end_matches("code"), "abi.json");
        let abi_file = compile_matches.value_of("ABI").or_else(|| {
            println!("ABI_PATH (obtained from INPUT): {}", abi_from_input);
            Some(abi_from_input.as_ref())
        });
        let abi_json = match abi_file {
            Some(abi_file_name) => Some(load_abi_json_string(abi_file_name)?),
            None => None
        };
        let out_file = compile_matches.value_of("OUT_FILE");

        let mut sources = Vec::new();
        for lib in compile_matches.values_of("LIB").unwrap_or_default() {
            let path = Path::new(lib);
            if !path.exists() {
                return Err(format!("File {} doesn't exist", lib));
            }
            sources.push(path);
        }
        let env_lib = env::var("TVM_LINKER_LIB_PATH").unwrap_or_default();
        if sources.is_empty() && !env_lib.is_empty() {
            println!("TVM_LINKER_LIB_PATH: {:?}", &env_lib);
            let path = Path::new(&env_lib);
            if !path.exists() {
                return Err(format!("File {} doesn't exist", &env_lib));
            }
            sources.push(path);
        }
            
        let path = Path::new(input);
        if !path.exists() {
            return Err(format!("File {} doesn't exist", input));
        }
        sources.push(path);

        let mut prog = Program::new(
            ParseEngine::new(sources, abi_json)?
        );

        match compile_matches.value_of("GENKEY") {
            Some(file) => {
                let pair = KeypairManager::new();
                pair.store_public(&(file.to_string() + ".pub"));
                pair.store_secret(file);
                prog.set_keypair(pair.drain());
            },
            None => match compile_matches.value_of("SETKEY") {
                Some(file) => {
                    let pair = KeypairManager::from_secret_file(file);
                    prog.set_keypair(pair.drain());
                },
                None => (),
            },
        };

        let debug = compile_matches.is_present("DEBUG");
        prog.set_language(compile_matches.value_of("LANGUAGE"));

        if debug {
           prog.debug_print();
        }

        let wc = compile_matches.value_of("WC")
            .map(|wc| i8::from_str_radix(wc, 10).unwrap_or(-1))
            .unwrap_or(-1);

        let ctor_params = compile_matches.value_of("CTOR_PARAMS");
        if ctor_params.is_some() && !abi_file.is_some() {
            let msg = "ABI is mandatory when CTOR_PARAMS is specified.";
            return Err(msg.to_string());
        }

        let data_filename = compile_matches.value_of("DATA");

        prog.compile_to_file_ex(wc, abi_file, ctor_params, out_file, debug, data_filename)?;

        if compile_matches.is_present("DEBUG_MAP") {
            let filename = compile_matches.value_of("DEBUG_MAP").unwrap();
            let file = File::create(filename).unwrap();
            serde_json::to_writer_pretty(file, &prog.dbgmap).unwrap();
        }

        return Ok(());
    }

    if let Some(m) = matches.subcommand_matches("disasm") {
        return disasm_command(m);
    }

    unreachable!()
}

fn parse_now(now: Option<&str>) -> Result<u32, String> {
    let now = match now {
        Some(now_str) => {
            u32::from_str_radix(now_str, 10)
                .map_err(|e| format!(r#"failed to parse "now" option: {}"#, e))?
        },
        None => get_now(),
    };
    Ok(now)
}

fn parse_ticktock(ticktock: Option<&str>) -> Result<Option<i8>, String> {
    let error = "invalid ticktock value: must be 0 for tick and -1 for tock.";
    if let Some(tt) = ticktock {
        let tt = i8::from_str_radix(tt, 10).map_err(|_| error.to_string())?;
        if tt != 0 && tt != -1 {
            Err(error.to_string())
        } else {
            Ok(Some(tt))
        }
    } else {
        Ok(None)
    }
}

fn run_init_subcmd(matches: &ArgMatches) -> Result<(), String> {
    let tvc = matches.value_of("INPUT").unwrap();
    let vars = matches.value_of("DATA").unwrap();
    let abi = matches.value_of("ABI").unwrap();
    set_initial_data(tvc, None, vars, abi)
}

fn decode_hex_string(hex_str: String) -> Result<(Vec<u8>, usize), String> {
    if hex_str.to_ascii_lowercase().starts_with('x') {
        let buf = SliceData::from_string(&hex_str[1..])
            .map_err(|_| format!("body {} is invalid literal slice", hex_str))?;
        Ok((buf.get_bytestring(0), buf.remaining_bits()))
    } else {
        let buf = hex::decode(&hex_str)
            .map_err(|_| format!("body {} is invalid hex string", hex_str))?;
        let buf_bits = buf.len() * 8;
        Ok((buf, buf_bits))
    }
}

fn run_test_subcmd(matches: &ArgMatches) -> Result<(), String> {
    let (body, sign) = match matches.value_of("BODY") {
        Some(hex_str) => {
            let mut hex_str = hex_str.to_string();

            let parse_results = match matches.value_of("SOURCE") {
                Some(source) => {
                    let path = Path::new(source);
                    if !path.exists() {
                        return Err(format!("File {} doesn't exist", source));
                    }
                    Some(ParseEngineResults::new(
                        ParseEngine::new(vec![path], None)?
                    ))
                },
                None => None
            };

            let line = Line::new(hex_str.as_str(), "", 0);
            let resolved = resolve_name(&line, |name| {
                let id = match &parse_results {
                    Some(parse_results) => parse_results.global_by_name(name),
                    None => None
                };
                id.map(|id| id.0)
            })
            .map_err(|e| format!("failed to resolve body {}: {}", hex_str, e))?;
            hex_str = resolved[0].text.clone();

            let (buf, buf_bits) = decode_hex_string(hex_str).unwrap();
            let body: SliceData = BuilderData::with_raw(buf, buf_bits)
                .map_err(|e| format!("failed to pack body in cell: {}", e))?
                .into_cell()
                .map_err(|e| format!("failed to pack body in cell: {}", e))?
                .into();
            (Some(body), Some(matches.value_of("SIGN")))
        },
        None => (build_body(matches)?, None),
    };

    let ticktock = parse_ticktock(matches.value_of("TICKTOCK"))?;
    let now = parse_now(matches.value_of("NOW"))?;

    let action_decoder = |body, is_internal| {
        let abi_file = matches.value_of("ABI_JSON");
        let method = matches.value_of("ABI_METHOD");
        if abi_file.is_some() && method.is_some() {
            let result = decode_body(abi_file.unwrap(), method.unwrap(), body, is_internal)
                .unwrap_or_default();
            println!("{}", result);
        }
    };
    
    let abi_json = matches.value_of("ABI_JSON");

    let _abi_contract = match abi_json {
        Some(abi_file) => Some(load_abi_contract(&load_abi_json_string(abi_file)?)?),
        None => None
    };

    let debug_map_filename = format!("{}{}", abi_json.map_or("debug_map.", |a| a.trim_end_matches("abi.json")), "map.json");

    println!("TEST STARTED");
    println!("body = {:?}", body);

    let mut msg_info = MsgInfo {
        balance: matches.value_of("INTERNAL"),
        src: matches.value_of("SRCADDR"),
        now,
        bounced: matches.is_present("BOUNCED"),
        body,
    };

    match matches.value_of("BODY_FROM_BOC") {
        Some(filename) => {
            let (mut root_slice, _) = load_stateinit(filename);
            let msg = Message::construct_from(&mut root_slice).expect("cannot read message from slice");
            msg_info.body = msg.body();
        }
        None => {}
    }

    let gas_limit = matches.value_of("GASLIMIT")
        .map(|v| i64::from_str_radix(v, 10))
        .transpose()
        .map_err(|e| format!("cannot parse gas limit value: {}", e))?;

    let mut trace_level = TraceLevel::None;
    if matches.is_present("TRACE") {
        trace_level = TraceLevel::Full;
    } else if matches.is_present("TRACE_MIN") {
        trace_level = TraceLevel::Minimal;
    }

    let input = matches.value_of("INPUT").unwrap();
    let addr_from_input = if hex::decode(input).is_ok() {
        input.to_owned()
    } else {
        std::iter::repeat("0").take(64).collect::<String>()
    };
    let address = matches.value_of("ADDRESS")
        .unwrap_or(&addr_from_input);

    let input = if input.contains(".tvc") {
        input.to_owned()
    } else {
        format!("{}.tvc", input)
    };
    call_contract(
        &input,
        address,
        matches.value_of("BALANCE"),
        msg_info,
        matches.value_of("CONFIG"),
        sign,
        ticktock,
        gas_limit,
        if matches.is_present("DECODEC6") { Some(action_decoder) } else { None },
        trace_level,
        debug_map_filename,
    )?;

    println!("TEST COMPLETED");
    return Ok(());
}

fn build_body(matches: &ArgMatches) -> Result<Option<SliceData>, String> {
    let mut mask = 0u8;
    let abi_file = matches.value_of("ABI_JSON").map(|m| {mask |= 1; m });
    let method_name = matches.value_of("ABI_METHOD").map(|m| {mask |= 2; m });
    let params = matches.value_of("ABI_PARAMS").map(|m| {mask |= 4; m });
    let header = matches.value_of("ABI_HEADER");
    if mask == 0x7 {
        let key_file = matches.value_of("SIGN").map(|path| {
            let pair = KeypairManager::from_secret_file(path);
            pair.drain()
        });
        let is_internal = matches.is_present("INTERNAL");
        let body: SliceData = build_abi_body(
            abi_file.unwrap(),
            method_name.unwrap(),
            params.unwrap(),
            header,
            key_file,
            is_internal
        )?.into_cell()
        .map_err(|e| format!("failed to pack body in cell: {}", e))?
        .into();
        Ok(Some(body))
    } else if mask == 0 {
        Ok(None)
    } else {
        Err("All ABI parameters must be supplied: ABI_JSON, ABI_METHOD, ABI_PARAMS".to_string())
    }
}
