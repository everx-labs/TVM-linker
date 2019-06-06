
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
use testcall::perform_contract_call;
use tvm::stack::BuilderData;

fn main() {
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
        )
    ).get_matches();

    if let Some(test_matches) = matches.subcommand_matches("test") {
        let body = match test_matches.value_of("BODY") {
            Some(hex_str) => {
                let mut hex_str = hex_str.to_string();

                if let Some(source) = test_matches.value_of("SOURCE") {
                    let file = File::open(source).expect("error opening source file");
                    let mut reader = BufReader::new(file);
                    let mut parser = ParseEngine::new();
                    parser.parse_code(&mut reader, true).expect("error");

                    hex_str = resolve_name(&hex_str, |name| {
                        parser.general_by_name(name).map(|id| id.0)
                    }).expect("error: body has invalid format");
                }

                let buf = hex::decode(&hex_str).expect("error: invalid hex string");
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
            test_matches.is_present("DECODEC6")
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
            .expect("error opening source file"), 
        matches.value_of("LIB")
            .map(|val| vec![val])
            .unwrap_or(vec![])
            .iter().map(|lib| File::open(lib).expect("error opening lib file"))
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

    let node_data = match matches.value_of("DATA") {
        Some(data) => Some(BuilderData::with_raw(hex::decode(data).unwrap(), data.len()*4)),
        None => None,
    };

    if matches.is_present("MESSAGE") {
        let mut suffix = "".to_owned();
        if matches.is_present("DATA") {
            suffix.push_str("-");
            suffix.push_str(matches.value_of("DATA").unwrap());
        }
        suffix.push_str(".boc");

        let re = Regex::new(r"\.[^.]+$").unwrap();
        let output_file = re.replace(matches.value_of("INPUT").unwrap(), suffix.as_str());
        
        compile_real_ton(prog.entry(), &BuilderData::from(&prog.data().unwrap()), node_data, &output_file, matches.is_present("INIT"));
        return;
    } else {
        prog.compile_to_file().expect("Error");
    }    
}
