
#[macro_use]
extern crate clap;
extern crate ed25519_dalek;
#[macro_use]
extern crate lazy_static;
extern crate rand;
extern crate regex;
extern crate sha2;
extern crate ton_block;
#[macro_use]
extern crate tvm;

mod program;
mod real_ton;
mod stdlib;
mod testcall;

use ed25519_dalek::Keypair;
use program::{Program, calc_func_id};
use rand::rngs::OsRng;
use regex::Regex;
use real_ton::{ decode_boc, compile_real_ton };
use sha2::Sha512;
use std::str;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::collections::HashMap;
use testcall::perform_contract_call;
use tvm::stack::BuilderData;

const FUNC_SUFFIX_AUTH: &str = "_authorized";

fn update_code_dict(prog: &mut Program, func_name: &String, func_body: &String, func_id: &mut i32) {
    if func_name == ".data" {
        prog.data = parse_data(func_body.as_str());       
    }
    else if func_name != "" {
        let name = func_name.to_owned();
        let mut signed = false;
        if let Some(index) = name.find(FUNC_SUFFIX_AUTH) {
            if (index + FUNC_SUFFIX_AUTH.len()) == name.len() {
                signed = true;
            }
        }
        let id = calc_func_id(name.as_str());
        assert_eq!(prog.code.insert(id, func_body.clone()), None);
        assert_eq!(prog.xrefs.insert(name, id), None);
        prog.signed.insert(id, signed);
        *func_id = *func_id + 1;
    }
}

fn parse_data(section: &str) -> BuilderData {
    let mut data = BuilderData::new();
    let data_buf = hex::decode(section.trim()).unwrap();
    let data_bits = data_buf.len() * 8;
    data.append_reference(BuilderData::with_raw(data_buf, data_bits));
    data
}

fn replace_labels (l: &String, xrefs: &mut HashMap<String, u32>) -> String {
    let mut result = "".to_owned();
    let mut ll = l.to_owned();

    let re = Regex::new(r"\$[A-Za-z0-9_]+\$").unwrap();
    loop {
        ll = match re.find(&ll) {
            None => {
                result.push_str(&ll);
                break result;
            }
            Some(mt) => {
                result.push_str(ll.get(0..mt.start()).unwrap());
                match xrefs.get(ll.get(mt.start()+1..mt.end()-1).unwrap()) {
                    Some(num) => {
                        let num_str = num.to_string();
                        result.push_str (&num_str);
                    }
                    None => { result.push_str ("???"); }
                }
                ll.get(mt.end()..).unwrap().to_owned()
            }
        }
    }
}

fn parse_code(prog: &mut Program, file_name: &str) {
    let f = File::open(file_name).expect("error: cannot load source file");
    let file = BufReader::new(&f);

    let globl_regex = Regex::new(r"^\t\.globl\t([a-zA-Z0-9_]+)").unwrap();
    let data_regex = Regex::new(r"^\t\.data").unwrap();
    let label_regex = Regex::new(r"^[.a-zA-Z0-9_]+:").unwrap();
    let dotted_regex = Regex::new(r"^\t*[.]").unwrap();

    let mut func_body: String = "".to_owned();
    let mut func_name: String = "".to_owned();
    let mut func_id: i32 = 1;

    for line in file.lines() {
        let l = line.unwrap();
        if globl_regex.is_match(&l) { 
            update_code_dict (prog, &func_name, &func_body, &mut func_id);
            func_name = "".to_owned();
            func_body = "".to_owned(); 

            for cap in globl_regex.captures_iter (&l) {
                func_name = cap[1].to_owned();
            }
            continue;
        }

        if data_regex.is_match(&l) {
            update_code_dict (prog, &func_name, &func_body, &mut func_id);
            func_name = ".data".to_owned();
            func_body = "".to_owned();
            continue;
        }

        if label_regex.is_match(&l) { continue; }
        if dotted_regex.is_match(&l) { continue; }

        let l_with_numbers = replace_labels (&l, &mut prog.xrefs);

        func_body.push_str (&l_with_numbers);
        func_body.push_str ("\n");
    }

    update_code_dict (prog, &func_name, &func_body, &mut func_id);
}

fn debug_print_program(prog: &Program) {
    println!("Entry point:\n-----------------\n{}\n-----------------", prog.get_entry());
    println!("Contract functions:\n-----------------");
    for (k,v) in &prog.xrefs {
        println! ("Function {:10}: id={}", k, v);
    }
    for (k,v) in &prog.code {
        println! ("Function {}\n-----------------\n{}\n-----------------", k, v);
    }    
    println! ("Dictionary of methods:\n-----------------\n{}", prog.get_method_dict());
}

fn generate_keypair() -> Keypair {
    Keypair::generate::<Sha512, _>(&mut OsRng::new().unwrap())
}

fn main() {
    let matches = clap_app! (tvm_loader =>
        (version: "0.1")
        (about: "Links TVM assembler file, loads and executes it in testing environment")
        (@arg PRINT_PARSED: --debug "Prints debug info: xref table and parsed assembler sources")
        (@arg DECODE: --decode "Decodes real TON message")
        (@arg MESSAGE: --message "Builds TON message for the contract in INPUT")
        (@arg INIT: --init "Packs code into TON State Init message")
        (@arg DATA: --data +takes_value "Supplies data to contract in hex format (empty data by default)")
        (@arg INPUT: +required +takes_value "TVM assembler source file")
        (@arg ENTRY_POINT: +takes_value "Function name of the contract's entry point")
        (@arg GEN_KEYPAIR: --("gen-keypair") +takes_value conflicts_with[SET_KEYPAIR] "Generates new keypair for the contract and saves it to the file")
        (@arg SET_KEYPAIR: --("set-keypair") +takes_value conflicts_with[GEN_KEYPAIR] "Loads existing keypair from the file")
        (@subcommand test =>
            (about: "execute contract in test environment")
            (version: "0.1")
            (author: "tonlabs")
            (@arg BODY: --body +takes_value "Body for external inbound message (hex string)")
        )
    ).get_matches();

    if matches.is_present("DECODE") {
        decode_boc(matches.value_of("INPUT").unwrap());
        return
    }

    let mut prog = Program::new();
    if matches.is_present("INPUT") {
        parse_code (&mut prog, matches.value_of("INPUT").unwrap());
        parse_code (&mut prog, matches.value_of("INPUT").unwrap());
    }

    prog.set_entry(matches.value_of("ENTRY_POINT")).expect("Error");

    match matches.value_of("GEN_KEYPAIR") {
        Some(file) => {
            let keys = generate_keypair();
            let bytes = keys.to_bytes();
            prog.set_keypair(keys);
            let mut file = File::create(file.to_string()).expect("error: cannot create key file");
            file.write_all(&bytes).unwrap();
        },
        None => match matches.value_of("SET_KEYPAIR") {
            Some(file) => {
                let mut file = File::open(file.to_string()).expect("error: cannot create key file");
                let mut keys_buf = vec![];
                file.read_to_end(&mut keys_buf).unwrap();
                prog.set_keypair(Keypair::from_bytes(&keys_buf).expect("error: cannot read key file"));
            },
            None => (),
        },
   };
   
    if matches.is_present("PRINT_PARSED") {
        debug_print_program(&prog);        
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
        
        compile_real_ton(prog.get_entry(), &prog.data, node_data, &output_file, matches.is_present("INIT"));
        return;
    } else {
        prog.compile_to_file().expect("Error");
    }

    if let Some(matches) = matches.subcommand_matches("test") {
        let body = match matches.value_of("BODY") {
            Some(hex_str) => {
                let buf = hex::decode(hex_str).expect("error: invalid hex string");
                let buf_bits = buf.len() * 8;
                Some(BuilderData::with_raw(buf, buf_bits).into())
            },
            None => None,
        };
        println!("test started: body = {:?}", body);
        perform_contract_call(&prog, body);
        println!("Test completed");
    }
}
