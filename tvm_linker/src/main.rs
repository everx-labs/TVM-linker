
#[macro_use]
extern crate clap;
extern crate ed25519_dalek;
#[macro_use]
extern crate lazy_static;
extern crate rand;
extern crate regex;
extern crate sha2;
extern crate simplelog;
extern crate ton_block;
#[macro_use]
extern crate tvm;
#[macro_use]
extern crate log;

mod keyman;
mod parser;
mod program;
mod real_ton;
mod resolver;
mod methdict;
mod testcall;

use keyman::KeypairManager;
use parser::ParseEngine;
use program::Program;
use real_ton::{ decode_boc, compile_real_ton };
use regex::Regex;
use resolver::resolve_name;
use std::fs::File;
use std::io::{BufReader};
use std::panic;
use testcall::perform_contract_call;
use tvm::stack::BuilderData;



fn main() {
    let default_panic_handler = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        if let Some(s) = panic_info.payload().downcast_ref::<String>() {
            println!("{}", s);
        } else {
            default_panic_handler(panic_info);
        }
    }));

    let matches = clap_app! (tvm_loader =>
        (version: "0.1")
        (author: "tonlabs")
        (about: "Links TVM assembler file, loads and executes it in testing environment")
        (@arg DEBUG: --debug "Prints debug info: xref table and parsed assembler sources")
        (@arg DECODE: --decode "Decodes real TON message")
        (@arg MESSAGE: --message "Builds TON message for the contract in INPUT")
        (@arg INIT: --init "Packs code into TON State Init message")
        (@arg DATA: --data +takes_value "Supplies data to contract in hex format (empty data by default)")
        (@arg INPUT: +required +takes_value "TVM assembler source file or contract name if used with test subcommand")
        (@arg LIB: --lib +takes_value "Standard library source file")
        (@arg GENKEY: --genkey +takes_value conflicts_with[SETKEY] "Generates new keypair for the contract and saves it to the file")
        (@arg SETKEY: --setkey +takes_value conflicts_with[GENKEY] "Loads existing keypair from the file")
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
    ).get_matches();

    if let Some(test_matches) = matches.subcommand_matches("test") {
        let body = match test_matches.value_of("BODY") {
            Some(hex_str) => {
                let mut hex_str = hex_str.to_string();
                let mut parser = ParseEngine::new();

                if let Some(source) = test_matches.value_of("SOURCE") {
                    let file = File::open(source).expect("error opening source file");
                    let mut reader = BufReader::new(file);
                    parser.parse_code(&mut reader, true).expect("error");
                }

                hex_str = resolve_name(&hex_str, |name| {
                    parser.global_by_name(name).map(|id| id.0)
                }).expect(&format!("error: failed to resolve body {}", hex_str));

                let buf = hex::decode(&hex_str).map_err(|_| format!("body {} is invalid hex string", hex_str)).expect("error");
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
        return;
    }

    if matches.is_present("DECODE") {
        decode_boc(matches.value_of("INPUT").unwrap());
        return
    }

    let mut parser = ParseEngine::new();
    parser.parse(
        File::open(matches.value_of("INPUT").unwrap())
            .map_err(|e| format!("cannot open source file: {}", e))
            .expect("error"), 
        matches.value_of("LIB")
            .map(|val| vec![val])
            .unwrap_or(vec![])
            .iter().map(|lib| File::open(lib).map_err(|e| format!("cannot open library file: {}", e)).expect("error"))
            .collect(),
    ).expect("error");

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
 
    if matches.is_present("MESSAGE") {
        let msg_body = match matches.value_of("DATA") {
            Some(data) => {
                let buf = hex::decode(data).unwrap();
                let len = buf.len() * 8;
                Some(BuilderData::with_raw(buf, len).into())
            },
            None => None,
        };
        let mut suffix = String::new();
        suffix += "-msg";
        if matches.is_present("INIT") {
        suffix += "-init";
        }
        suffix += ".boc";

        let re = Regex::new(r"\.[^.]+$").unwrap();
        let msg_file = re.replace(matches.value_of("INPUT").unwrap(), suffix.as_str());
        
        compile_real_ton(prog.compile_to_state().expect("error"), msg_body, &msg_file, matches.is_present("INIT"));
        return;
    } else {
        prog.compile_to_file().expect("error");
    }    
}
