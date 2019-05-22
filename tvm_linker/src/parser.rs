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

    pub fn parse(&mut self, source_file: &str, lib_files: Vec<&str>) -> Result<(), String> {
        for file in lib_files {
            self.parse_code(file, false)?;
            self.parse_code(file, true)?;
        }

        self.parse_code(source_file, false)?;
        self.parse_code(source_file, true)?;
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

    fn parse_code(&mut self, file: &str, parse_selector: bool) -> Result<(), String> {
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
        let mut func_id: i32 = 1;

        let file = File::open(file).map_err(|e| format!("cannot read source file: {}", e))?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let l = line.unwrap();
            //println!("{}", l);
            if globl_regex.is_match(&l) { 
                self.update(&section_name, &func_name, &func_body, &mut func_id)?;
                section_name = GLOBL.to_owned();
                func_body = "".to_owned(); 
                func_name = globl_regex.captures(&l).unwrap().get(1).unwrap().as_str().to_owned();
            } else if data_regex.is_match(&l) {
                self.update(&section_name, &func_name, &func_body, &mut func_id)?;
                section_name = DATA.to_owned();
                func_name = "".to_owned();
                func_body = "".to_owned();
            } else if selector_regex.is_match(&l) {
                if !parse_selector { continue; }
                self.update(&section_name, &func_name, &func_body, &mut func_id)?;
                section_name = SELECTOR.to_owned();
                func_name = "".to_owned();
                func_body = "".to_owned();
            } else if internal_regex.is_match(&l) {
                self.update(&section_name, &func_name, &func_body, &mut func_id)?;
                section_name = INTERNAL.to_owned();
                func_body = "".to_owned();
                func_name = internal_regex.captures(&l).unwrap().get(1).unwrap().as_str().to_owned();
            } else if label_regex.is_match(&l) { 
                continue;             
            } else if alias_regex.is_match(&l) {
                let cap = alias_regex.captures(&l).unwrap();
                self.aliases.insert(
                    cap.get(1).unwrap().as_str().to_owned(), 
                    i32::from_str_radix(cap.get(2).unwrap().as_str(), 10)
                        .map_err(|_| format!("line: '{}': failed to parse id", l))?, 
                );
            } else if dotted_regex.is_match(&l) { 
                 
            } else {
                let l_with_numbers = self.replace_labels(&l);
                func_body.push_str(&l_with_numbers);
                func_body.push_str("\n");
            }
        }

        self.update(&section_name, &func_name, &func_body, &mut func_id)?;
        ok!()
    }

    fn update(&mut self, section: &str, func: &str, body: &str, id: &mut i32) -> Result<(), String> {
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
                self.generals.insert(func_id, body.trim_end().to_string());
                self.xrefs.insert(func.to_string(), func_id);
                self.signed.insert(func_id, signed);
            },
            INTERNAL => {
                let f_id = self.aliases.get(func).unwrap_or(id);
                self.internals.insert(*f_id, body.trim_end().to_string());
                self.intrefs.insert(func.to_string(), *f_id);
            },
            _ => (),
        }
        *id = *id + 1;
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
        let mut result = String::new();
        let mut line = line;
        let re = Regex::new(r"\$:?[A-Za-z0-9_]+\$").unwrap();
        loop {
            line = match re.find(line) {
                None => {
                    result.push_str(line);
                    break result;
                }
                Some(mt) => {
                    let parts: Vec<&str> = re.split(line).collect();
                    result.push_str(parts.get(0).unwrap_or(&""));
                    let pointer = line.get(mt.start()+1..mt.end()-1).expect("failed to extract label from line");
                    let id_name = {
                        if pointer.starts_with(":") {
                            self.intrefs.get(pointer).map(|id| id.to_string())
                        } else {
                            self.xrefs.get(pointer).map(|id| id.to_string())
                        }
                    }.unwrap_or("???".to_string());
                    result.push_str(&id_name);
                    parts.get(1).unwrap_or(&"")
                }
            };
        }
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
        for (k, v) in &self.xrefs {
            println! ("Function {:30}: id={:08X}, sign-check={:?}", k, v, self.signed.get(&v).unwrap());
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

    #[test]
    fn test_parser_testlib() {
        let mut parser = ParseEngine::new();
        assert_eq!(parser.parse("./tests/pbank.s", vec!["./test.tvm"]), ok!());
    }

    #[test]
    fn test_parser_stdlib() {
        let mut parser = ParseEngine::new();
        assert_eq!(parser.parse("./tests/pbank.s", vec!["./stdlib.tvm"]), ok!());
    }
}