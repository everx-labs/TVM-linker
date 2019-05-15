
#[macro_use]
extern crate clap;
#[macro_use]
extern crate lazy_static;
extern crate regex;
#[macro_use]
extern crate tvm;
extern crate ton_block;

use regex::Regex;


mod real_ton;
use real_ton::{ decode_boc, compile_real_ton };

use std::str;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::collections::HashMap;

mod program;
use program::Program;

use tvm::bitstring::Bitstring;
mod stdlib;
use stdlib::*;

mod testcall;
use testcall::perform_contract_call;

fn update_code_dict (prog: &mut Program, func_name: &String, func_body: &String, func_id: &mut i32) {
    if func_name == ".data" {
        let value = func_body.trim();
        prog.data = Bitstring::create(hex::decode (value).unwrap(), value.len()*4);
    }
    else if func_name != "" {
        prog.xrefs.insert (func_name.clone(), *func_id);
        prog.code.insert (*func_id, func_body.clone());
        *func_id = *func_id + 1;
    }
}

fn replace_labels (l: &String, xrefs: &mut HashMap<String,i32>) -> String {
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

fn parse_code (prog: &mut Program, file_name: &str) {
    let f = File::open(file_name).unwrap();
    let file = BufReader::new(&f);

    let globl_regex = Regex::new(r"^\t\.globl\t([a-zA-Z0-9_]+)").unwrap();
    let data_regex = Regex::new(r"^\t\.data").unwrap();
    let label_regex = Regex::new(r"^[.a-zA-Z0-9_]+:").unwrap();
    let dotted_regex = Regex::new(r"^\t*[.]").unwrap();

    let mut func_body: String = "".to_owned();
    let mut func_name: String = "".to_owned();
    let mut func_id: i32 = 0;

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
        (@arg MAIN: +required +takes_value "Function name to call")
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

    if matches.is_present("PRINT_PARSED") {
        for (k,v) in &prog.xrefs {
            println! ("Function {}: id={}", k, v);
        }

        for (k,v) in &prog.code {
            println! ("Function {}\n-----------------\n{}\n-----------------", k, v);
        }

        println! ("");
    }

    let mut main_id = None;
    if matches.is_present("MAIN") {
        let main_name = matches.value_of("MAIN").unwrap();
        match prog.xrefs.get (main_name) {
            None => {
                println! ("Main method {} is not found in source code", main_name);
                return
            }
            Some(v) => main_id = Some(*v)
        }
    }

    let node_data_option;
    let mut node_data = None;
    if matches.is_present("DATA") {
        let data = matches.value_of("DATA").unwrap();
        node_data_option = Bitstring::create(hex::decode(data).unwrap(),data.len()*4);
        node_data = Some (&node_data_option);
    }

    if matches.is_present("MESSAGE") {
        let mut suffix = "".to_owned();
        if matches.is_present("DATA") {
            suffix.push_str("-");
            suffix.push_str(matches.value_of("DATA").unwrap());
        }
        suffix.push_str(".boc");

        let re = Regex::new(r"\.[^.]+$").unwrap();
        let output_file = re.replace(matches.value_of("INPUT").unwrap(), suffix.as_str());

        let main_code;
        match main_id {
            None => main_code = "???",
            Some(id) => main_code = prog.code.get(&id).unwrap().as_str()
        }
        compile_real_ton(main_code, &prog.data, &node_data, &output_file, matches.is_present("INIT"));
        return
    }
    else if matches.is_present("INPUT") && matches.is_present("MAIN") {
        let mut serialized_code: Vec<(i32,String)> = [].to_vec();
        for (k,v) in &prog.code {
            serialized_code.push ((*k,v.to_string()));
        }
        perform_contract_call(&serialized_code, main_id.unwrap(), &node_data)
    }
}
