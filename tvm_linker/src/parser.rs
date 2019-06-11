use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use tvm::stack::BuilderData;
use resolver::resolve_name;

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

const PATTERN_GLOBL:    &'static str = r"^[\t\s]*\.globl[\t\s]+([a-zA-Z0-9_]+)";
const PATTERN_DATA:     &'static str = r"^[\t\s]*\.data";
const PATTERN_INTERNAL: &'static str = r"^[\t\s]*\.internal[\t\s]+(:[a-zA-Z0-9_]+)";
const PATTERN_SELECTOR: &'static str = r"^[\t\s]*\.selector";
const PATTERN_ALIAS:    &'static str = r"^[\t\s]*\.internal-alias (:[a-zA-Z0-9_]+),[\t\s]+(-?\d+)";
const PATTERN_LABEL:    &'static str = r"^[.a-zA-Z0-9_]+:";
const PATTERN_PARAM:    &'static str = r"^\t+[.]";

const GLOBL:    &'static str = ".globl";
const INTERNAL: &'static str = ".internal";
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

    pub fn parse<T: Read + Seek>(&mut self, source: T, libs: Vec<T>) -> Result<(), String> {
        for lib_buf in libs {
            let mut reader = BufReader::new(lib_buf);
            self.parse_code(&mut reader, true)?;
            reader.seek(SeekFrom::Start(0))
                .map_err(|e| format!("error while seeking lib file: {}", e))?;
            self.parse_code(&mut reader, false)?;
        }
        let mut reader = BufReader::new(source);
        self.parse_code(&mut reader, true)?;
        reader.seek(SeekFrom::Start(0))
            .map_err(|e| format!("error while seeking source file: {}", e))?;
        self.parse_code(&mut reader, false)?;

        if self.entry_point.is_empty() {
            return Err("Selector not found".to_string());
        }
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

    pub fn internal_by_name(&self, name: &str) -> Option<(i32, String)> {
        let id = self.intrefs.get(name)?;
        let body = self.internals.get(id).map(|v| v.to_owned())?;
        Some((*id, body))
    }

    pub fn general_by_name(&self, name: &str) -> Option<(u32, String)> {
        let id = self.xrefs.get(name)?;
        let body = self.generals.get(id).map(|v| v.to_owned())?;
        Some((*id, body))
    }

    pub fn generals(&self) -> &HashMap<u32, String> {
        &self.generals
    }

    pub fn signed(&self) -> &HashMap<u32, bool> {
        &self.signed
    }

    pub fn parse_code<R: BufRead>(&mut self, reader: &mut R, first_pass: bool) -> Result<(), String> {
        let globl_regex = Regex::new(PATTERN_GLOBL).unwrap();
        let internal_regex = Regex::new(PATTERN_INTERNAL).unwrap();
        let selector_regex = Regex::new(PATTERN_SELECTOR).unwrap();
        let data_regex = Regex::new(PATTERN_DATA).unwrap();
        let label_regex = Regex::new(PATTERN_LABEL).unwrap();
        let dotted_regex = Regex::new(PATTERN_PARAM).unwrap();
        let alias_regex = Regex::new(PATTERN_ALIAS).unwrap();

        let mut section_name: String = String::new();
        let mut func_body: String = "".to_owned();
        let mut func_name: String = "".to_owned();

        let mut l = String::new();
        while reader.read_line(&mut l)
            .map_err(|_| "error while reading line")? != 0 {
            if globl_regex.is_match(&l) { 
                self.update(&section_name, &func_name, &func_body, first_pass)?;
                section_name = GLOBL.to_owned();
                func_body = "".to_owned(); 
                func_name = globl_regex.captures(&l).unwrap().get(1).unwrap().as_str().to_owned();
            } else if data_regex.is_match(&l) {
                self.update(&section_name, &func_name, &func_body, first_pass)?;
                section_name = DATA.to_owned();
                func_name = "".to_owned();
                func_body = "".to_owned();
            } else if selector_regex.is_match(&l) {                
                self.update(&section_name, &func_name, &func_body, first_pass)?;
                if first_pass { 
                    section_name.clear();
                } else {
                    section_name = SELECTOR.to_owned();
                }
                func_name = "".to_owned();
                func_body = "".to_owned();
            } else if internal_regex.is_match(&l) {
                self.update(&section_name, &func_name, &func_body, first_pass)?;
                section_name = INTERNAL.to_owned();
                func_body = "".to_owned();
                func_name = internal_regex.captures(&l).unwrap().get(1).unwrap().as_str().to_owned();
            } else if label_regex.is_match(&l) { 
                            
            } else if alias_regex.is_match(&l) {
                let cap = alias_regex.captures(&l).unwrap();
                self.aliases.insert(
                    cap.get(1).unwrap().as_str().to_owned(), 
                    i32::from_str_radix(cap.get(2).unwrap().as_str(), 10)
                        .map_err(|_| format!("line: '{}': failed to parse id", l))?, 
                );                
            } else if dotted_regex.is_match(&l) { 
                 
            } else {
                let l_with_numbers = if first_pass { l.to_owned() } else { self.replace_labels(&l) };
                func_body.push_str(&l_with_numbers);
            }
            l.clear();
        }

        self.update(&section_name, &func_name, &func_body, first_pass)?;
        ok!()
    }

    fn update(&mut self, section: &str, func: &str, body: &str, first_pass: bool) -> Result<(), String> {
        match section {
            DATA => self.parse_data(body),
            SELECTOR => {
                if self.entry_point.is_empty() {
                    self.entry_point = body.trim_end().to_string();
                } else {
                    return Err("Another selector found".to_string());
                }
            },
            GLOBL => {
                let mut signed = false;
                if let Some(index) = func.find(FUNC_SUFFIX_AUTH) {
                    if (index + FUNC_SUFFIX_AUTH.len()) == func.len() {
                        signed = true;
                    }
                }
                let func_id = calc_func_id(func);
                let prev = self.generals.insert(func_id, body.trim_end().to_string());
                if first_pass && prev.is_some() {
                    Err(format!("global function with id = {} already exist", func_id))?;
                }
                self.xrefs.insert(func.to_string(), func_id);
                self.signed.insert(func_id, signed);
            },
            INTERNAL => {
                let f_id = self.aliases.get(func).ok_or(format!("id for '{}' not found", func))?;
                let prev = self.internals.insert(*f_id, body.trim_end().to_string());
                if first_pass && prev.is_some() {
                    Err(format!("internal function with id = {} already exist", *f_id))?;
                }
                self.intrefs.insert(func.to_string(), *f_id);
            },
            _ => (),
        }
        ok!()
    }

    fn parse_data(&mut self, section: &str) {
        let mut data = BuilderData::new();
        let data_buf = hex::decode(section.trim()).unwrap();
        let data_bits = data_buf.len() * 8;
        data.append_reference(BuilderData::with_raw(data_buf, data_bits));
        self.data = data;
    }

    fn replace_labels(&mut self, line: &str) -> String {
        resolve_name(line, |name| self.xrefs.get(name).map(|id| id.clone()))
            .or_else(|_| resolve_name(line, |name| self.intrefs.get(name).map(|id| id.clone())))
            .unwrap_or(line.to_string())
    }

    pub fn debug_print(&self) {
        let line = "--------------------------";
        println!("Entry point:\n{}\n{}\n{}", line, self.entry(), line);
        println!("General-purpose functions:\n{}", line);
        for (k, v) in &self.xrefs {
            println! ("Function {:30}: id={:08X}, sign-check={:?}", k, v, self.signed.get(&v).unwrap());
        }
        for (k, v) in &self.generals {
            println! ("Function {:08X}\n{}\n{}\n{}", k, line, v, line);
        }        
        println!("{}\nInternal functions:\n{}", line, line);
        for (k, v) in &self.intrefs {
            println! ("Function {:30}: id={:08X}", k, v);
        }
        for (k, v) in &self.internals {
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


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    fn test_parser_testlib() {
        let mut parser = ParseEngine::new();
        let pbank_file = File::open("./tests/pbank.s").unwrap();
        let test_file = File::open("./tests/test.tvm").unwrap();
        assert_eq!(parser.parse(pbank_file, vec![test_file]), ok!());
        parser.debug_print();
    }

    #[test]
    fn test_parser_stdlib() {
        let mut parser = ParseEngine::new();
        let pbank_file = File::open("./tests/pbank.s").unwrap();
        let test_file = File::open("./stdlib.tvm").unwrap();
        assert_eq!(parser.parse(pbank_file, vec![test_file]), ok!());
    }
}