use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use tvm::stack::BuilderData;

pub struct ParseEngine {
    xrefs: HashMap<String, u32>,
    intrefs: HashMap<String, i32>,
    aliases: HashMap<String, i32>,
    generals: HashMap<u32, String>,
    internals: HashMap<i32, String>,
    data: BuilderData,
    signed: HashMap<u32, bool>,
    entry_point: String,
}

const PATTERN_GLOBL:    &'static str = r"^\t\.globl\t([a-zA-Z0-9_]+)";
const PATTERN_DATA:     &'static str = r"^\t\.data";
const PATTERN_INTERNAL: &'static str = r"^\t\.internal\t:([a-zA-Z0-9_]+)";
const PATTERN_SELECTOR: &'static str = r"^\t\.selector";
const PATTERN_ALIAS:    &'static str = r"^\t\.internal-alias :([a-zA-Z0-9_]+),[\t\s]+(-?\d+)";
const PATTERN_LABEL:    &'static str = r"^[.a-zA-Z0-9_]+:";
const PATTERN_PARAM:    &'static str = r"^\t*[.]";

//const GLOBL:    &'static str = ".globl";
//const INTERNAL: &'static str = ".internal";
const DATA:     &'static str = ".data";
const SELECTOR: &'static str = ".selector";

const FUNC_SUFFIX_AUTH: &'static str = "_authorized";

impl ParseEngine {

    pub fn new() -> Self {
        ParseEngine {
            xrefs:      HashMap::new(), 
            intrefs:    HashMap::new(), 
            aliases:    HashMap::new(),
            generals:   HashMap::new(), 
            internals:  HashMap::new(),
            data:       BuilderData::new(), 
            signed:     HashMap::new(),
            entry_point: String::new(),
        }
    }

    pub fn parse(&mut self, source_file: &str, lib_files: Vec<&str>) -> Result<(), String> {
        for file in lib_files {
            self.parse_code(file)?;
            self.parse_code(file)?;
        }

        self.parse_code(source_file)?;
        self.parse_code(source_file)?;
        ok!()
    }

    pub fn data(&self) -> &BuilderData {
        &self.data
    }

    pub fn entry(&self) -> &str {
        &self.entry_point
    }

    pub fn internals(&self) -> &HashMap<i32, String> {
        &self.internals
    }

    pub fn generals(&self) -> &HashMap<u32, String> {
        &self.generals
    }

    pub fn signed(&self) -> &HashMap<u32, bool> {
        &self.signed
    }

    fn parse_code(&mut self, file: &str) -> Result<(), String> {
        let globl_regex = Regex::new(PATTERN_GLOBL).unwrap();
        let internal_regex = Regex::new(PATTERN_INTERNAL).unwrap();
        let selector_regex = Regex::new(PATTERN_SELECTOR).unwrap();
        let data_regex = Regex::new(PATTERN_DATA).unwrap();
        let label_regex = Regex::new(PATTERN_LABEL).unwrap();
        let dotted_regex = Regex::new(PATTERN_PARAM).unwrap();
        let alias_regex = Regex::new(PATTERN_ALIAS).unwrap();

        let mut func_body: String = "".to_owned();
        let mut func_name: String = "".to_owned();
        let mut func_id: i32 = 1;

        let file = File::open(file).map_err(|e| format!("cannot read source file: {}", e))?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let l = line.unwrap();
            if globl_regex.is_match(&l) { 
                self.update(&func_name, &func_body, &mut func_id);
                func_name = "".to_owned();
                func_body = "".to_owned(); 

                for cap in globl_regex.captures_iter(&l) {
                    func_name = cap[1].to_owned();
                }
            } else if data_regex.is_match(&l) {
                self.update(&func_name, &func_body, &mut func_id);
                func_name = DATA.to_owned();
                func_body = "".to_owned();
            } else if selector_regex.is_match(&l) {
                func_name = SELECTOR.to_owned();
                func_body = "".to_owned();
            } else if internal_regex.is_match(&l) {
                func_name = "".to_owned();
                func_body = "".to_owned();
            } else if label_regex.is_match(&l) { 
                continue; 
            } else if dotted_regex.is_match(&l) { 
                continue; 
            } else if alias_regex.is_match(&l) {

            }

            let l_with_numbers = self.replace_labels(&l);

            func_body.push_str(&l_with_numbers);
            func_body.push_str("\n");
        }

        self.update(&func_name, &func_body, &mut func_id);
        ok!()
    }

    fn update(&mut self, func_name: &String, func_body: &String, func_id: &mut i32) {
        if func_name == DATA {
            self.parse_data(func_body.as_str());
        } else if func_name != "" {
            let mut name = func_name.to_owned();
            let mut signed = false;
            if let Some(index) = name.find(FUNC_SUFFIX_AUTH) {
                if (index + FUNC_SUFFIX_AUTH.len()) == name.len() {
                    signed = true;
                    name.truncate(index + 1);
                }
            }
            let id = calc_func_id(name.as_str());
            self.generals.insert(id, func_body.trim_end().to_string());
            self.xrefs.insert(func_name.to_owned(), id);
            self.signed.insert(id, signed);
            *func_id = *func_id + 1;
        }
    }

    fn parse_data(&mut self, section: &str) {
        let mut data = BuilderData::new();
        let data_buf = hex::decode(section.trim()).unwrap();
        let data_bits = data_buf.len() * 8;
        data.append_reference(BuilderData::with_raw(data_buf, data_bits));
        self.data = data;
    }

    fn replace_labels(&mut self, l: &str) -> String {
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
                    match self.xrefs.get(ll.get(mt.start()+1..mt.end()-1).unwrap()) {
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

    pub fn debug_print(&self) {
        let line = "--------------------------";
        println!("Entry point:\n{}\n{}\n{}", line, self.entry(), line);
        println!("Contract functions:\n{}", line);
        for (k, v) in &self.xrefs {
            println! ("Function {:30}: id={:08X}, sign-check={:?}", k, v, self.signed.get(&v).unwrap());
        }
        for (k, v) in &self.generals {
            println! ("Function {:08X}\n{}\n{}\n{}", k, line, v, line);
        }
    }
}

pub fn calc_func_id(func_interface: &str) -> u32 {
    let mut hasher = Sha256::new();
    hasher.input(func_interface.as_bytes());
    let mut id_bytes = [0u8; 4];
    id_bytes.copy_from_slice(&hasher.result()[..4]);
    u32::from_be_bytes(id_bytes)
} 