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
extern crate ton_abi as abi_json;
extern crate base64;
#[macro_use]
extern crate clap;
extern crate crc16;
extern crate ed25519;
extern crate ed25519_dalek;
extern crate failure;
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
mod keyman;
mod parser;
mod printer;
mod program;
mod resolver;
mod methdict;
mod testcall;
mod disasm;

use abi::{build_abi_body, decode_body, load_abi_json_string, load_abi_contract};
use clap::ArgMatches;
use failure::{format_err, bail};
use keyman::KeypairManager;
use parser::{ParseEngine, ParseEngineResults};
use program::{Program, get_now, save_to_file};
use resolver::resolve_name;
use ton_block::{Deserializable, Message, StateInit, Serializable, Account, MsgAddressInt, ExternalInboundMessageHeader};
use std::io::Write;
use std::{path::Path};
use testcall::{call_contract, MsgInfo, TraceLevel};
use ton_types::{BuilderData, SliceData, Result, Status, AccountId, BagOfCells, BocSerialiseMode};
use std::env;
use disasm::commands::disasm_command;
use ton_labs_assembler::Line;
use std::fs::File;
use std::str::FromStr;

fn main() -> std::result::Result<(), i32> {
    linker_main().map_err(|err_str| {
        println!("Error: {}", err_str);
        1
    })
}

fn linker_main() -> Status {
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
        (@subcommand replace_code =>
            (@setting AllowNegativeNumbers)
            (about: "Compile assembler code file and replace contract code with a new one.")
            (version: build_info.as_str())
            (author: "TON Labs")
            (@arg INPUT: +required +takes_value "TVM assembler source file")
            (@arg CONTRACT_PATH: +required +takes_value "Path to the file with the BOC of contract account state whose code should be replaced.")
            (@arg ABI: -a --("abi-json") +takes_value "Supplies contract abi to calculate correct function ids. If not specified abi can be loaded from file path obtained from <INPUT> path if it exists.")
            (@arg DEBUG_MAP: --("debug-map") +takes_value "Generates debug map file")
            (@arg LIB: --lib +takes_value ... number_of_values(1) "Standard library source file. If not specified lib is loaded from environment variable TVM_LINKER_LIB_PATH if it exists.")
            (@arg OUT_FILE: -o +takes_value "Output file name. If not specified the input file is rewritten.")
            (@arg TVC: --tvc "Changes command behaviour to work with stateInit TVC instead of account BOC.")
        )
        (@subcommand compile =>
            (@setting AllowNegativeNumbers)
            (about: "compile contract")
            (version: build_info.as_str())
            (author: "TON Labs")
            (@arg INPUT: +required +takes_value "TVM assembler source file")
            (@arg ABI: -a --("abi-json") +takes_value "Supplies contract abi to calculate correct function ids. If not specified abi can be loaded from file path obtained from <INPUT> path if it exists.")
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
            (@arg ADDRESS: --address +takes_value "Contract address, which can be obtained from the contract with `address(this)`. If not specified address can be obtained from the INPUT argument or set to zero.")
            (@arg ABI_JSON: -a --("abi-json") +takes_value conflicts_with[BODY] "Supplies json file with contract ABI")
            (@arg ABI_METHOD: -m --("abi-method") +takes_value conflicts_with[BODY] "Supplies the name of the calling contract method")
            (@arg ABI_PARAMS: -p --("abi-params") +takes_value conflicts_with[BODY] "Supplies ABI arguments for the contract method (can be passed via filename). Can be not specified for empty parameters.")
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
            (@subcommand text =>
                (about: "disassembles tvc's code into assembler text")
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
        return decode_boc(
            decode_matches.value_of("INPUT").unwrap(),
            decode_matches.is_present("TVC"),
        );
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
                let buf = hex::decode(data).map_err(|e| format_err!("data argument has invalid format: {}", e))?;
                let len = buf.len() * 8;
                let body: SliceData = BuilderData::with_raw(buf, len)?.into_cell()?.into();
                Some(body)
            },
            None => {
                build_body(msg_matches)?
            },
        };

        return build_message(
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
                bail!("File {} doesn't exist", lib);
            }
            sources.push(path);
        }
        let env_lib = env::var("TVM_LINKER_LIB_PATH").unwrap_or_default();
        if sources.is_empty() && !env_lib.is_empty() {
            println!("TVM_LINKER_LIB_PATH: {:?}", &env_lib);
            let path = Path::new(&env_lib);
            if !path.exists() {
                bail!("File {} doesn't exist", &env_lib);
            }
            sources.push(path);
        }

        let path = Path::new(input);
        if !path.exists() {
            bail!("File {} doesn't exist", input);
        }
        sources.push(path);
        let mut prog = Program::new(
            ParseEngine::new(sources, abi_json)?
        );

        match compile_matches.value_of("GENKEY") {
            Some(file) => {
                let pair = KeypairManager::new();
                pair.store_public(&(file.to_string() + ".pub"))?;
                pair.store_secret(file)?;
                prog.set_keypair(pair.drain());
            },
            None => if let Some(file) = compile_matches.value_of("SETKEY") {
                let pair = KeypairManager::from_secret_file(file)
                    .ok_or_else(|| format_err!("Failed to read keypair"))?;
                prog.set_keypair(pair.drain());
            },
        };

        let debug = compile_matches.is_present("DEBUG");
        prog.set_language(compile_matches.value_of("LANGUAGE"));

        if debug {
           prog.debug_print();
        }

        let wc = compile_matches.value_of("WC")
            .map(|wc| wc.parse::<i8>().unwrap_or(-1))
            .unwrap_or(-1);

        let ctor_params = compile_matches.value_of("CTOR_PARAMS");
        if ctor_params.is_some() && abi_file.is_none() {
            let msg = "ABI is mandatory when CTOR_PARAMS is specified.";
            bail!(msg);
        }

        let data_filename = compile_matches.value_of("DATA");

        prog.compile_to_file_ex(wc, abi_file, ctor_params, out_file, debug, data_filename)?;

        if compile_matches.is_present("DEBUG_MAP") {
            let filename = compile_matches.value_of("DEBUG_MAP").unwrap();
            let file = File::create(filename)?;
            serde_json::to_writer_pretty(file, &prog.dbgmap)?;
        }

        return Ok(());
    }

    if let Some(m) = matches.subcommand_matches("disasm") {
        return disasm_command(m);
    }

    if let Some(matches) = matches.subcommand_matches("replace_code") {
        return  replace_command(matches);
    }

    unreachable!()
}

fn replace_command(matches: &ArgMatches) -> Status {
    let input = matches.value_of("INPUT").unwrap();
    let abi_from_input = format!("{}{}", input.trim_end_matches("code"), "abi.json");
    let abi_file = matches.value_of("ABI").or_else(|| {
        println!("ABI_PATH (obtained from INPUT): {}", abi_from_input);
        Some(abi_from_input.as_ref())
    });
    let abi_json = match abi_file {
        Some(abi_file_name) => Some(load_abi_json_string(abi_file_name)?),
        None => None
    };
    let out_file = matches.value_of("OUT_FILE");

    let mut sources = Vec::new();
    for lib in matches.values_of("LIB").unwrap_or_default() {
        let path = Path::new(lib);
        if !path.exists() {
            bail!("File {} doesn't exist", lib);
        }
        sources.push(path);
    }
    let env_lib = env::var("TVM_LINKER_LIB_PATH").unwrap_or_default();
    if sources.is_empty() && !env_lib.is_empty() {
        println!("TVM_LINKER_LIB_PATH: {:?}", &env_lib);
        let path = Path::new(&env_lib);
        if !path.exists() {
            bail!("File {} doesn't exist", &env_lib);
        }
        sources.push(path);
    }

    let path = Path::new(input);
    if !path.exists() {
        bail!("File {} doesn't exist", input);
    }
    sources.push(path);

    let mut prog = Program::new(
        ParseEngine::new(sources, abi_json)?
    );

    let code = prog.compile_asm(false)?;

    let input_path = matches.value_of("CONTRACT_PATH").unwrap();
    let out_file = out_file.unwrap_or(input_path);
    if matches.is_present("TVC") {
        let mut state_init = StateInit::construct_from_file(input_path)?;
        state_init.set_code(code);
        state_init.write_to_file(out_file)?;
    } else {
        let mut account = Account::construct_from_file(input_path)?;
        match account.state_init() {
            Some(_) => {
                account.set_code(code);
            }
            None => {
                bail!("Account doesn't contain stateInit.")
            }
        }
        account.write_to_file(out_file)?;
    }
    println!("Result saved to file: {}", out_file);
    if matches.is_present("DEBUG_MAP") {
        let filename = matches.value_of("DEBUG_MAP").unwrap();
        let file = File::create(filename)?;
        serde_json::to_writer_pretty(file, &prog.dbgmap)?;
    }

    Ok(())
}

fn parse_now(now: Option<&str>) -> Result<u32> {
    let now = match now {
        Some(now_str) => {
            now_str.parse::<u32>().map_err(|e| format_err!("failed to parse \"now\" option: {}", e))?
        },
        None => get_now(),
    };
    Ok(now)
}

fn parse_ticktock(ticktock: Option<&str>) -> Result<Option<i8>> {
    let error = "invalid ticktock value: must be 0 for tick and -1 for tock.";
    if let Some(tt) = ticktock {
        let tt = tt.parse::<i8>().map_err(|e| format_err!("{}: {}", error, e))?;
        if tt != 0 && tt != -1 {
            bail!(error)
        } else {
            Ok(Some(tt))
        }
    } else {
        Ok(None)
    }
}

fn set_initial_data(tvc: &str, data: &str, abi: &str) -> Status {
    let mut state_init = std::fs::OpenOptions::new().read(true).open(tvc)
        .map_err(|e| format_err!("unable to open contract file {}: {}", tvc, e))?;
    let abi = load_abi_json_string(abi)?;

    let mut contract_image =
        ton_sdk::ContractImage::from_state_init(&mut state_init)
            .map_err(|e| format_err!("unable to load contract image: {}", e))?;

    contract_image.update_data(data, &abi)
        .map_err(|e| format_err!("unable to update contract image data: {}", e))?;

    save_to_file(contract_image.state_init(), None, 0)?;
    Ok(())
}

fn run_init_subcmd(matches: &ArgMatches) -> Status {
    let tvc = matches.value_of("INPUT").unwrap();
    let data = matches.value_of("DATA").unwrap();
    let abi = matches.value_of("ABI").unwrap();
    set_initial_data(tvc, data, abi)
}

fn decode_hex_string(hex_str: String) -> Result<(Vec<u8>, usize)> {
    if hex_str.to_ascii_lowercase().starts_with('x') {
        let buf = SliceData::from_string(&hex_str[1..])
            .map_err(|_| format_err!("body {} is invalid literal slice", hex_str))?;
        Ok((buf.get_bytestring(0), buf.remaining_bits()))
    } else {
        let buf = hex::decode(&hex_str)
            .map_err(|_| format_err!("body {} is invalid hex string", hex_str))?;
        let buf_bits = buf.len() * 8;
        Ok((buf, buf_bits))
    }
}

fn decode_boc(filename: &str, is_tvc: bool) -> Status {
    let (mut root_slice, orig_bytes) = program::load_stateinit(filename)?;

    println!("Encoded: {}\n", hex::encode(orig_bytes));
    if is_tvc {
        let state = StateInit::construct_from(&mut root_slice)?;
        println!("Decoded:\n{}", printer::state_init_printer(&state));
    } else {
        let msg = Message::construct_from(&mut root_slice)?;
        println!("Decoded:\n{}", printer::msg_printer(&msg)?);
    }
    Ok(())
}

fn run_test_subcmd(matches: &ArgMatches) -> Status {
    let (body, sign) = match matches.value_of("BODY") {
        Some(hex_str) => {
            let mut hex_str = hex_str.to_string();

            let parse_results = match matches.value_of("SOURCE") {
                Some(source) => {
                    let path = Path::new(source);
                    if !path.exists() {
                        bail!("File {} doesn't exist", source);
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
            .map_err(|e| format_err!("failed to resolve body {}: {}", hex_str, e))?;
            hex_str = resolved.text.clone();

            let (buf, buf_bits) = decode_hex_string(hex_str)?;
            let body: SliceData = BuilderData::with_raw(buf, buf_bits)
                .map_err(|e| format_err!("failed to pack body in cell: {}", e))?
                .into_cell()
                .map_err(|e| format_err!("failed to pack body in cell: {}", e))?
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
        if let Some(abi_file) = abi_file {
            if let Some(method) = method {
                let result = decode_body(abi_file, method, body, is_internal)
                    .unwrap_or_default();
                println!("{}", result);
            }
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

    if let Some(filename) = matches.value_of("BODY_FROM_BOC") {
        let (mut root_slice, _) = program::load_stateinit(filename)?;
        let msg = Message::construct_from(&mut root_slice)?;
        msg_info.body = msg.body();
    }

    let gas_limit = matches.value_of("GASLIMIT")
        .map(|v| v.parse::<i64>())
        .transpose()?;

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
        "0".repeat(64)
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
        None
    )?;

    println!("TEST COMPLETED");
    Ok(())
}

fn build_body(matches: &ArgMatches) -> Result<Option<SliceData>> {
    let mut mask = 0u8;
    let abi_file = matches.value_of("ABI_JSON").map(|m| {mask |= 1; m });
    let method_name = matches.value_of("ABI_METHOD").map(|m| {mask |= 2; m });
    let params = matches.value_of("ABI_PARAMS");
    let header = matches.value_of("ABI_HEADER");
    if mask == 0x3 {
        let key_file = match matches.value_of("SIGN") {
            Some(path) => {
                let pair = KeypairManager::from_secret_file(path)
                    .ok_or_else(|| format_err!("Failed to read keypair."))?;
                Some(pair.drain())
            },
            _ => None
        };
        let params = params.map_or(Ok("{}".to_owned()), |params|
            if params.find('{').is_none() {
                std::fs::read_to_string(params)
                    .map_err(|e| format_err!("failed to load params from file: {}", e))
            } else {
                Ok(params.to_owned())
            }
        )?;
        let is_internal = matches.is_present("INTERNAL");
        let body: SliceData = build_abi_body(
            abi_file.unwrap(),
            method_name.unwrap(),
            &params,
            header,
            key_file,
            is_internal
        )?.into_cell()?.into();
        Ok(Some(body))
    } else if mask == 0 {
        Ok(None)
    } else {
        bail!("All ABI parameters must be supplied: ABI_JSON, ABI_METHOD")
    }
}

fn build_message(
    address_str: &str,
    wc: Option<&str>,
    body: Option<SliceData>,
    pack_code: bool,
    suffix: &str,
) -> Status {
    let wc = match wc {
        Some(w) => w.parse::<i8>()?,
        None => -1,
    };
    println!("contract address {}", address_str);
    let dest_address = MsgAddressInt::with_standart(
        None,
        wc,
        AccountId::from_str(address_str)?
    )?;

    let msg_hdr = ExternalInboundMessageHeader {
        dst: dest_address,
        ..Default::default()
    };
    let mut msg = Message::with_ext_in_header(msg_hdr);
    let state = if pack_code {
        msg.set_state_init(program::load_from_file(&format!("{}.tvc", address_str))?);
    }
    if let Some(body) = body {
        msg.set_body(body);
    }

    let root_cell = msg.serialize()?;
    let boc = BagOfCells::with_root(&root_cell);
    let mut bytes = Vec::new();
    let mode = BocSerialiseMode::Generic { index: false, crc: true, cache_bits: false, flags: 0 };
    boc.write_to_ex(&mut bytes, mode, None, Some(4))?;

    println!("Encoded msg: {}", hex::encode(&bytes));

    let output_file_name = address_str.get(0..8).unwrap_or("00000000").to_string() + suffix;
    let mut f = File::create(&output_file_name)?;
    f.write_all(&bytes)?;

    println!("boc file created: {}", output_file_name);
    Ok(())
}
