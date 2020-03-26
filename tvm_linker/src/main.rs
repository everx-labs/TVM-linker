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
extern crate ton_abi as abi_json;
extern crate base64;
#[macro_use]
extern crate clap;
extern crate crc16;
extern crate ed25519_dalek;
#[macro_use]
extern crate lazy_static;
extern crate rand;
extern crate regex;
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

use abi::{build_abi_body, decode_body};
use clap::ArgMatches;
use initdata::set_initial_data;
use keyman::KeypairManager;
use parser::ParseEngine;
use program::Program;
use real_ton::{ decode_boc, compile_message };
use resolver::resolve_name;
use std::fs::File;
use std::io::{BufReader, Read};
use std::time::SystemTime;
use testcall::perform_contract_call;
use ton_types::{BuilderData, SliceData};

fn main() -> Result<(), i32> {
    println!(
        "TVM linker {}\nCOMMIT_ID: {}\nBUILD_DATE: {}\nCOMMIT_DATE: {}\nGIT_BRANCH: {}",
        env!("CARGO_PKG_VERSION"),
        env!("BUILD_GIT_COMMIT"),
        env!("BUILD_TIME") ,
        env!("BUILD_GIT_DATE"),
        env!("BUILD_GIT_BRANCH")
    );
    linker_main().map_err(|err_str| {
        println!("Error: {}", err_str);
        1
    })
}

fn linker_main() -> Result<(), String> {
    let build_info = match option_env!("BUILD_INFO") {
        Some(s) => s,
        None => "",
    };
    let matches = clap_app! (tvm_linker =>        
        (version: &*format!("0.1 ({})", build_info))
        (author: "TONLabs")
        (about: "Linker for TVM assembly")
        (@subcommand decode => 
            (about: "Decode real TON message")
            (version: "0.1")
            (author: "TONLabs")
            (@arg INPUT: +required +takes_value "BOC file")
            (@arg TVC: --tvc "BOC file is tvc file")
        )
        (@subcommand compile =>
            (@setting AllowNegativeNumbers)
            (about: "compile contract")
            (version: "0.1")
            (author: "TONLabs")
            (@arg INPUT: +required +takes_value "TVM assembler source file")
            (@arg ABI: -a --("abi-json") +takes_value "Supplies contract abi to calculate correct function ids")
            (@arg GENKEY: --genkey +takes_value conflicts_with[SETKEY] "Generates new keypair for the contract and saves it to the file")
            (@arg SETKEY: --setkey +takes_value conflicts_with[GENKEY] "Loads existing keypair from the file")
            (@arg WC: -w +takes_value "Workchain id used to print contract address, -1 by default.")            
            (@arg DEBUG: --debug "Prints debug info: xref table and parsed assembler sources")
            (@arg LIB: --lib +takes_value ... number_of_values(1) "Standard library source file")
        )
        (@subcommand test =>
            (about: "execute contract in test environment")
            (version: "0.1")
            (author: "TONLabs")
            (@arg SOURCE: -s --source +takes_value "Contract source file")
            (@arg BODY: --body +takes_value "Body for external inbound message (hex string)")
            (@arg SIGN: --sign +takes_value "Signs body with private key from defined file")
            (@arg TRACE: --trace "Prints last command name, stack and registers after each executed TVM command")
            (@arg DECODEC6: --("decode-c6") "Prints last command name, stack and registers after each executed TVM command")
            (@arg INTERNAL: --internal +takes_value "Emulates inbound internal message with value instead of external message")
            (@arg SRCADDR: --src +takes_value "Supplies message source address")
            (@arg NOW: --now +takes_value "Supplies transaction creation unixtime")
            (@arg TICKTOCK: --ticktock +takes_value conflicts_with[BODY] "Emulates ticktock transaction in masterchain, 0 for tick and -1 for tock")
            (@arg INPUT: +required +takes_value "TVM assembler source file or contract name if used with test subcommand")
            (@arg ABI_JSON: -a --("abi-json") +takes_value conflicts_with[BODY] "Supplies json file with contract ABI")
            (@arg ABI_METHOD: -m --("abi-method") +takes_value conflicts_with[BODY] "Supplies the name of the calling contract method")
            (@arg ABI_PARAMS: -p --("abi-params") +takes_value conflicts_with[BODY] "Supplies ABI arguments for the contract method")
            (@arg ABI_HEADER: -h --("abi-header") +takes_value conflicts_with[BODY] conflicts_with[INTERNAL] "Supplies ABI header")
        )
        (@subcommand message =>
            (@setting AllowNegativeNumbers)
            (about: "generate external inbound message for the blockchain")
            (version: "0.1")
            (author: "TONLabs")
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
            (@arg INPUT: +required +takes_value "Path to compiled smart contract file")
            (@arg DATA: +required +takes_value "Set of public variables with values in json format")
            (@arg ABI: +required +takes_value "Path to smart contract ABI file")
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
        let mut parser = ParseEngine::new();
        let abi_json = 
            if compile_matches.is_present("ABI") {
                let abi_file_name = compile_matches.value_of("ABI").unwrap();
                let mut f = File::open(abi_file_name).map_err(|e| format!("cannot open abi file: {}", e))?;
                let mut abi = String::new(); 
                Some(f.read_to_string(&mut abi).map(|_| abi).map_err(|e| format!("failed to read abi: {}", e))?)
            } else { 
                None 
            };
            let libs = compile_matches.values_of("LIB")
                .unwrap_or_default()
                .map(|lib| {
                    File::open(lib)
                        .map_err(|e| format!("cannot open library file: {}", e))
                        .expect("error")
                })
                .collect();
            let source = File::open(compile_matches.value_of("INPUT").unwrap())
                .map_err(|e| format!("cannot open source file: {}", e))?;

        parser.parse(source, libs, abi_json)?;
        let mut prog = Program::new(parser);

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
       
        if compile_matches.is_present("DEBUG") {
           prog.debug_print();        
        }

        let wc = compile_matches.value_of("WC")
            .map(|wc| i8::from_str_radix(wc, 10).unwrap_or(-1))
            .unwrap_or(-1);
         
        prog.compile_to_file(wc)?;
        return Ok(());
    }

    unreachable!()
}

fn run_init_subcmd(matches: &ArgMatches) -> Result<(), String> {
    let tvc = matches.value_of("INPUT").unwrap();
    let vars = matches.value_of("DATA").unwrap();
    let abi = matches.value_of("ABI").unwrap();
    set_initial_data(tvc, None, vars, abi)
}


fn run_test_subcmd(test_matches: &ArgMatches) -> Result<(), String> {
    let (body, sign) = match test_matches.value_of("BODY") {
        Some(hex_str) => {
            let mut hex_str = hex_str.to_string();
            let mut parser = ParseEngine::new();

            if let Some(source) = test_matches.value_of("SOURCE") {
                let file = File::open(source)
                    .map_err(|e| format!("cannot opening source file: {}", e))?;
                let mut reader = BufReader::new(file);
                parser.parse_code(&mut reader, true)?;
            }

            hex_str = resolve_name(&hex_str, |name| {
                parser.global_by_name(name).map(|id| id.0)
            })
            .map_err(|e| format!("failed to resolve body {}: {}", hex_str, e))?;

            let buf = hex::decode(&hex_str)
                .map_err(|_| format!("body {} is invalid hex string", hex_str))?;
            let buf_bits = buf.len() * 8;
            let body: SliceData = BuilderData::with_raw(buf, buf_bits)
                .map_err(|e| format!("failed to pack body in cell: {}", e))?
                .into();
            (Some(body), Some(test_matches.value_of("SIGN")))
        },
        None => (build_body(test_matches)?, None),
    };

    let ticktock = test_matches.value_of("TICKTOCK");
    if ticktock.is_some() {
        //check for correct value of ticktock argument
        let error_msg = "invalid ticktock value: enter 0 for tick and -1 for tock.".to_string();
        let val = i8::from_str_radix(ticktock.unwrap(), 10).map_err(|_| error_msg.clone())?;
        if val != 0 && val != -1 {
            Err(error_msg)?;
        }
    }
    
    let now = match test_matches.value_of("NOW") {
        Some(now_str) => {
            u32::from_str_radix(now_str, 10)
                .map_err(|e| format!("failed to parse \"now\" option: {}", e))?
        },
        None => SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32,
    };

    println!("TEST STARTED\nbody = {:?}", body);
    perform_contract_call(
        test_matches.value_of("INPUT").unwrap(), 
        body, 
        sign, 
        test_matches.is_present("TRACE"), 
        test_matches.is_present("DECODEC6"),
        test_matches.value_of("INTERNAL"),
        test_matches.value_of("TICKTOCK"),
        test_matches.value_of("SRCADDR"),
        now,
        |body, is_int| {
            let abi_file = test_matches.value_of("ABI_JSON");
            let method = test_matches.value_of("ABI_METHOD");
            if abi_file.is_some() && method.is_some() {
                let result = decode_body(abi_file.unwrap(), method.unwrap(), body, is_int)
                    .unwrap_or_default();
                println!("{}", result);
            }
        }
    );
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
        )?.into();            
        Ok(Some(body))
    } else if mask == 0 {
        Ok(None)
    } else {
        Err("All ABI parameters must be supplied: ABI_JSON, ABI_METHOD, ABI_PARAMS".to_string())
    }
}