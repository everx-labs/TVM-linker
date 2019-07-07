extern crate abi_json;
#[macro_use]
extern crate clap;
extern crate ed25519_dalek;
extern crate hex;
#[macro_use]
extern crate lazy_static;
extern crate rand;
extern crate regex;
extern crate serde_json;
extern crate sha2;
extern crate simplelog;
extern crate ton_block;
#[macro_use]
extern crate tvm;
#[macro_use]
extern crate log;

mod abi;
mod keyman;
mod parser;
mod program;
mod real_ton;
mod resolver;
mod methdict;
mod testcall;

use abi::build_abi_body;
use keyman::KeypairManager;
use parser::ParseEngine;
use program::Program;
use real_ton::{ decode_boc, compile_message };
use resolver::resolve_name;
use std::fs::File;
use std::io::{BufReader, Read};
use testcall::perform_contract_call;
use tvm::stack::BuilderData;

fn main() {
    if let Err(err_str) = linker_main() {
        println!("error: {}", err_str);
    }
}

fn linker_main() -> Result<(), String> {
    let matches = clap_app! (tvm_loader =>
        (version: "0.1")
        (author: "tonlabs")
        (about: "Links TVM assembler file, loads and executes it in testing environment")
        (@arg DEBUG: --debug "Prints debug info: xref table and parsed assembler sources")
        (@arg DECODE: --decode "Decodes real TON message")       
        (@arg INPUT: +required +takes_value "TVM assembler source file or contract name if used with test subcommand")
        (@arg LIB: --lib +takes_value "Standard library source file")
        (@arg GENKEY: --genkey +takes_value conflicts_with[SETKEY] "Generates new keypair for the contract and saves it to the file")
        (@arg SETKEY: --setkey +takes_value conflicts_with[GENKEY] "Loads existing keypair from the file")
        (@arg ABI: -a --("abi-json") +takes_value "Supplies contract abi to calculate correct function ids")
        (@subcommand test =>
            (about: "execute contract in test environment")
            (version: "0.1")
            (author: "tonlabs")
            (@arg SOURCE: -s --source +takes_value "contract source file")
            (@arg BODY: --body +takes_value "Body for external inbound message (hex string)")
            (@arg SIGN: --sign +takes_value "Signs body with private key from defined file")
            (@arg TRACE: --trace "Prints last command name, stack and registers after each executed TVM command")
            (@arg DECODEC6: --("decode-c6") "Prints last command name, stack and registers after each executed TVM command")
            (@arg INTERNAL: --internal +takes_value "Emulates inbound internal message with value instead of external message")
        )
        (@subcommand message =>
            (about: "generate external inbound message for the blockchain")
            (version: "0.1")
            (author: "tonlabs")
            (@arg INIT: -i --init "Generates constructor message with code and data of the contract")
            (@arg DATA: -d --data +takes_value "Supplies body for the message in hex format (empty data by default)")
            (@arg WORKCHAIN: -w --workchain +takes_value "Supplies workchain id for the contract address")
            (@arg ABI_JSON: -a --("abi-json") +takes_value conflicts_with[DATA] "Supplies json file with contract ABI")
            (@arg ABI_METHOD: -m --("abi-method") +takes_value conflicts_with[DATA] "Supplies the name of the calling contract method")
            (@arg ABI_PARAMS: -p --("abi-params") +takes_value conflicts_with[DATA] "Supplies ABI arguments for the contract method")
            (@arg ABI_SIGN: -s --("abi-sign") +takes_value conflicts_with[DATA] "Supplies private key file to sign encoded ABI body")
        )
    ).get_matches();

    if let Some(test_matches) = matches.subcommand_matches("test") {
        let body = match test_matches.value_of("BODY") {
            Some(hex_str) => {
                let mut hex_str = hex_str.to_string();
                let mut parser = ParseEngine::new();

                if let Some(source) = test_matches.value_of("SOURCE") {
                    let file = File::open(source).map_err(|e| format!("cannot opening source file: {}", e))?;
                    let mut reader = BufReader::new(file);
                    parser.parse_code(&mut reader, true)?;
                }

                hex_str = resolve_name(&hex_str, |name| {
                    parser.global_by_name(name).map(|id| id.0)
                }).map_err(|e| format!("failed to resolve body {}: {}", hex_str, e))?;

                let buf = hex::decode(&hex_str).map_err(|_| format!("body {} is invalid hex string", hex_str))?;
                let buf_bits = buf.len() * 8;
                Some(BuilderData::with_raw(buf, buf_bits).into())
            },
            None => None,
        };
        
        println!("TEST STARTED\nbody = {:?}", body);
        perform_contract_call(
            matches.value_of("INPUT").unwrap(), 
            body, 
            test_matches.value_of("SIGN"), 
            test_matches.is_present("TRACE"), 
            test_matches.is_present("DECODEC6"),
            test_matches.value_of("INTERNAL"),
        );
        println!("TEST COMPLETED");
        return ok!();
    }

    if matches.is_present("DECODE") {
        decode_boc(matches.value_of("INPUT").unwrap());
        return ok!();
    }

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
                Some(BuilderData::with_raw(buf, len).into())
            },
            None => {
                let mut mask = 0u8;
                let abi_file = msg_matches.value_of("ABI_JSON").map(|m| {mask |= 1; m });
                let method_name = msg_matches.value_of("ABI_METHOD").map(|m| {mask |= 2; m });
                let params = msg_matches.value_of("ABI_PARAMS").map(|m| {mask |= 4; m });

                if mask == 0x7 {
                    let key_file = msg_matches.value_of("ABI_SIGN").map(|path| {
                        let pair = KeypairManager::from_secret_file(path);
                        pair.drain()
                    });
                    Some(build_abi_body(abi_file.unwrap(), method_name.unwrap(), params.unwrap(), key_file)?.into())
                } else if mask == 0 {
                    None
                } else {
                    return Err("All ABI parameters must be supplied: ABI_JSON, ABI_METHOD, ABI_PARAMS".to_string());
                }
            },
        };
        
        return compile_message(
            matches.value_of("INPUT").unwrap(), 
            msg_matches.value_of("WORKCHAIN"), 
            msg_body, 
            msg_matches.is_present("INIT"), 
            &suffix,
        )
    }

    let mut parser = ParseEngine::new();
    let abi_json = 
        if matches.is_present("ABI") {
            let abi_file_name = matches.value_of("ABI").unwrap();
            let mut f = File::open(abi_file_name).map_err(|e| format!("cannot open abi file: {}", e))?;
            let mut abi = String::new(); 
            Some(f.read_to_string(&mut abi).map(|_| abi).map_err(|e| format!("failed to read abi: {}", e))?)
        } else { 
            None 
        };
    parser.parse(
        File::open(matches.value_of("INPUT").unwrap())
            .map_err(|e| format!("cannot open source file: {}", e))?,
        matches.value_of("LIB")
            .map(|val| vec![val])
            .unwrap_or(vec![])
            .iter().map(|lib| File::open(lib).map_err(|e| format!("cannot open library file: {}", e)).expect("error"))
            .collect(),
        abi_json,
    )?;

    let mut prog = Program::new(parser);

    match matches.value_of("GENKEY") {
        Some(file) => {
            let pair = KeypairManager::new();
            pair.store_public(&(file.to_string() + ".pub"));
            pair.store_secret(file);
            prog.set_keypair(pair.drain());
        },
        None => match matches.value_of("SETKEY") {
            Some(file) => {
                let pair = KeypairManager::from_secret_file(file);
                prog.set_keypair(pair.drain());
            },
            None => (),
        },
   };
   
    if matches.is_present("DEBUG") {
       prog.debug_print();        
    }
     
    prog.compile_to_file()?;
    ok!()
}
