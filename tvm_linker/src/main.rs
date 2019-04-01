#[macro_use]
extern crate tvm;
extern crate ton_block;
extern crate contract_api;
extern crate regex;

#[macro_use]
extern crate clap;

use std::str;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::collections::HashMap;
use regex::Regex;

use contract_api::executor::prepare_methods;
use contract_api::test_framework::{test_case_with_ref, Expects};

mod real_ton;
use real_ton::{ make_boc, decode_boc, compile_real_ton };

mod program;
use program::Program;

use tvm::stack::{
        Stack,
        SliceData,
        CellData,
        BuilderData,
        StackItem,
        IntegerData,
};

use tvm::stack::dictionary::HashmapE;
use tvm::stack::dictionary::HashmapType;
use tvm::bitstring::Bitstring;

use ton_block::{
    Serializable,
    ExternalInboundMessageHeader,
    MsgAddressInt,
    Message
};
use tvm::types::AccountId;
use std::sync::Arc;

pub struct TestABIContract {
    dict: SliceData,        // dictionary of methods
}

/// Constructs test contract to implement dictionary of methods
pub trait TestContractCode {
    fn new(&[(i32,String)]) -> Self;
    fn get_contract_code(&self) -> &str;
    fn get_methods(&self) -> SliceData;
}

impl TestContractCode for TestABIContract {
    fn get_contract_code(&self) -> &str {
        CODE_CONTRACT
    }    

    fn get_methods(&self) -> SliceData {
        self.dict.clone()
    }

    fn new(raw_methods: &[(i32, String)]) -> Self {
        let dict = prepare_methods(&[
            (-1i8,  INBOUND_EXTERNAL_PARSER.to_string()),
            // (0,     MAIN),
        ]);

        let methods = prepare_methods(raw_methods);

        let key = 1i8.write_to_new_cell().unwrap();
        let mut dict = HashmapE::with_data(8, dict);
        dict.set(key.into(), methods).unwrap();
        TestABIContract { dict: dict.get_data() }
    }
}

pub const MAIN_ID: i32 = 0x6D61696E;

static INBOUND_EXTERNAL_PARSER: &str = "
    ; s0 - msg body: slice
    ; s1 - msg header: cell
    ; s2 - gram balance of msg: int
    ; s3 - gram balance of contract: int

    ; parse body
    LDU 8       ; load version
    NIP         ; drop version
    LDU 32      ; load func id
    POP s4      ; drop gram balance of contract
    POP s2      ; drop gram balance of msg
    DROP        ; drop header
    CALL 1
";

static CODE_CONTRACT: &str = "
    ; s0 - func_id i8
    ; s1.. - other data
    PUSHREFSLICE        ; dictionary of methods in first reference (what if code more than 1023 bits: 0-ref - continue of code)
    OVER
    ISNEG
    PUSHCONT {          ; if func_id negative - direct call to method
        PUSHINT 8
        DICTIGETJMP     ; execute method and return
        THROW 51
    }
    PUSHCONT {          ; get dictionary with methods
        PUSHINT 8
        DICTIGET
        THROWIFNOT 52   ; no dictionary of methods
        PUSHINT 32
        DICTUGETJMP     ; execute method and return
        THROW 51
    }
    IFELSE
";

fn create_inbound_body(a: i32, b: i32, func_id: i32) -> Arc<CellData> {
    let mut builder = BuilderData::new();
    let version: u8 = 0;
    version.write_to(&mut builder).unwrap();
    func_id.write_to(&mut builder).unwrap();
    a.write_to(&mut builder).unwrap();
    b.write_to(&mut builder).unwrap();
    builder.into()
}

fn create_external_inbound_msg(dst_addr: &AccountId, body: Arc<CellData>) -> Message {
    let mut hdr = ExternalInboundMessageHeader::default();
    hdr.dst = MsgAddressInt::with_standart(None, -1, dst_addr.clone()).unwrap();
    let mut msg = Message::with_ext_in_header(hdr);
    msg.body = Some(body.into());
    msg
}

fn perform_contract_call(raw_methods: &[(i32,String)], func_id: i32, a: i32, b: i32) {
    let mut stack = Stack::new();
    let body_cell = create_inbound_body(a, b, func_id);
    let msg_cell = StackItem::Cell(
        create_external_inbound_msg(
            &AccountId::from([0x11; 32]), 
            body_cell.clone()
        ).write_to_new_cell().unwrap().into()
    );
    stack
        .push(int!(0))
        .push(int!(0))
        .push(msg_cell.clone())
        .push(StackItem::Slice(SliceData::from(body_cell))) 
        .push(int!(-1));

    let contract = TestABIContract::new(raw_methods);

    test_case_with_ref(&contract.get_contract_code(), contract.get_methods())
        .with_stack(stack).expect_success().expect_print_stack();
}

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

    let mut prog: Program = Program { xrefs: HashMap::new(), code: HashMap::new(), data: Bitstring::default() };
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

    let mut node_data_option;
    let mut node_data = None;
    if matches.is_present("DATA") {
        let data = matches.value_of("DATA").unwrap();
        node_data_option = Bitstring::create(hex::decode(data).unwrap(),data.len()*4);
        node_data = Some (&node_data_option);
    }

    if matches.is_present("MESSAGE") {
        let re = Regex::new(r"\.[^.]+$").unwrap();
        let output_file = re.replace(matches.value_of("INPUT").unwrap(), ".boc");
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
        perform_contract_call(&serialized_code, main_id.unwrap(), 0, 0)
    }
}
