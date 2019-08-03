use abi::gen_abi_id;
use abi_json::Contract;
use regex::Regex;
use resolver::resolve_name;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use tvm::stack::{BuilderData, IBitstring, IntegerData, SliceData};
use tvm::stack::integer::serialization::{Encoding, SignedIntegerBigEndianEncoding};
use tvm::stack::serialization::Serializer;
use tvm::stack::dictionary::{HashmapE, HashmapType};
pub type Ptr = i64;

pub fn ptr_to_builder(n: Ptr) -> Result<BuilderData, String> {
    let mut b = BuilderData::new();
    b.append_i64(n).map_err(|_| format!("failed to serialize an i64 to buidler"))?;
    Ok(b)
}

enum ObjectType {
    None,
    Function((u32, String)),
    Data { addr: Ptr, values: Vec<DataValue>, persistent: bool },
}
impl From<&str> for ObjectType {
    fn from(stype: &str) -> ObjectType {
        match stype {
            "function" => ObjectType::Function((0, String::new())),
            "object" => ObjectType::Data{ addr: 0, values: vec![], persistent: false },
            _ => ObjectType::None,
        }
    }
}
enum DataValue {
    Empty,
    Number((IntegerData, usize)),
}

impl DataValue {
    pub fn write(&self) -> BuilderData {
        let mut b = BuilderData::new();
        match self {
            DataValue::Number(ref intgr) => {
                let encoding = SignedIntegerBigEndianEncoding::new(257);
                let mut dest_vec = vec![];
                encoding.try_serialize(&intgr.0).unwrap().into_bitstring_with_completion_tag(&mut dest_vec);
                b.append_bitstring(&dest_vec[..]).unwrap();
                b
            },
            DataValue::Empty => b,
        }
    }
}

struct Object {
    pub name: String,
    pub size: usize,
    pub index: usize,
    pub dtype: ObjectType,
}

impl Object {
    pub fn new(name: String, stype: &str) -> Self {
        Object {
            name,
            size: 0,
            index: 0,
            dtype: ObjectType::from(stype),
        }
    }
}

impl Default for Object {
    fn default() -> Self {
        Object::new(String::new(), "")
    }
}

pub struct ParseEngine {
    xrefs: HashMap<String, u32>,
    intrefs: HashMap<String, i32>,
    aliases: HashMap<String, i32>,
    globals: HashMap<String, Object>,
    internals: HashMap<i32, String>,
    entry_point: String,
    globl_base: Ptr,
    globl_ptr: Ptr,
    pub persistent_base: Ptr,
    persistent_ptr: Ptr,
    abi: Option<Contract>,
}

const PATTERN_GLOBL:    &'static str = r"^[\t\s]*\.globl[\t\s]+([a-zA-Z0-9_\.]+)";
const PATTERN_DATA:     &'static str = r"^[\t\s]*\.data";
const PATTERN_INTERNAL: &'static str = r"^[\t\s]*\.internal[\t\s]+(:[a-zA-Z0-9_]+)";
const PATTERN_SELECTOR: &'static str = r"^[\t\s]*\.selector";
const PATTERN_ALIAS:    &'static str = r"^[\t\s]*\.internal-alias (:[a-zA-Z0-9_]+),[\t\s]+(-?\d+)";
const PATTERN_GLBLBASE: &'static str = r"^[\t\s]*\.global-base[\t\s]+([0-9]+)";
const PATTERN_PERSBASE: &'static str = r"^[\t\s]*\.persistent-base[\t\s]+([0-9]+)";
const PATTERN_LABEL:    &'static str = r"^[\.a-zA-Z0-9_]+:";
//const PATTERN_GLOBLSTART: &'static str = r"^([a-zA-Z0-9_]+):";
const PATTERN_PARAM:    &'static str = r"^[\t\s]+\.([a-zA-Z0-9_]+),?[\t\s]*([a-zA-Z0-9_]*)";
const PATTERN_TYPE:     &'static str = r"^[\t\s]*\.type[\t\s]+([a-zA-Z0-9_\.]+),[\t\s]*@([a-zA-Z]+)";
const PATTERN_SIZE:     &'static str = r"^[\t\s]*\.size[\t\s]+([a-zA-Z0-9_\.]+),[\t\s]*([\.a-zA-Z0-9_]+)";
const PATTERN_COMM:     &'static str = r"^[\t\s]*\.comm[\t\s]+([a-zA-Z0-9_\.]+),[\t\s]*([0-9]+),[\t\s]*([0-9]+)";
const PATTERN_IGNORED:  &'static str = r"^[\t\s]+\.(p2align|align|text|file|ident|section)";

const GLOBL:    &'static str = ".globl";
const INTERNAL: &'static str = ".internal";
const SELECTOR: &'static str = ".selector";

//const FUNCTION_TYPENAME:&'static str = "function";
const DATA_TYPENAME:    &'static str = "object";

const PERSISTENT_DATA_SUFFIX: &'static str = "_persistent";

const PUBKEY_NAME: &'static str = "tvm_public_key";
const SCI_NAME: &'static str = "tvm_contract_info";

const WORD_SIZE: Ptr = 8;
const OFFSET_GLOBL_DATA: Ptr = 8;
const OFFSET_PERS_DATA: Ptr = 16;

impl ParseEngine {

    pub fn new() -> Self {
        ParseEngine {
            xrefs:      HashMap::new(), 
            intrefs:    HashMap::new(), 
            aliases:    HashMap::new(),
            globals:    HashMap::new(), 
            internals:  HashMap::new(),
            entry_point: String::new(),
            globl_base: 0,
            globl_ptr: 0,
            persistent_base: 0,
            persistent_ptr: 0,
            abi: None,
        }
    }

    pub fn parse<T: Read + Seek>(&mut self, source: T, libs: Vec<T>, abi_json: Option<String>) -> Result<(), String> {
        if let Some(s) = abi_json {
            self.abi = Some(Contract::load(s.as_bytes()).map_err(|e| format!("cannot parse contract abi: {:?}", e))?);
        }

        self.preinit()?;

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

    pub fn data(&self) -> SliceData {
        self.build_data()
    }

    pub fn entry(&self) -> &str {
        &self.entry_point
    }

    pub fn internals(&self) -> &HashMap<i32, String> {
        &self.internals
    }

    pub fn internal_name(&self, id: i32) -> Option<String> {
        self.intrefs.iter().find(|i| *i.1 == id).map(|i| i.0.clone())
    }

    pub fn internal_by_name(&self, name: &str) -> Option<(i32, String)> {
        let id = self.intrefs.get(name)?;
        let body = self.internals.get(id).map(|v| v.to_owned())?;
        Some((*id, body))
    }

    pub fn globals(&self) -> HashMap<u32, String> {
        let mut funcs = HashMap::new();
        let iter = self.globals.iter().filter_map(|item| {
            match &item.1.dtype {
                ObjectType::Function(ref func) => Some(func),
                _ => None,
            }
        });
        for i in iter {
            funcs.insert(i.0, i.1.clone());
        }
        funcs
    }

    pub fn global_name(&self, id: u32) -> Option<String> {
        self.globals.iter().find(|item| {
            match item.1.dtype {
                ObjectType::Function(ref func) => func.0 == id,
                _ => false,
            }
        })
        .map(|i| i.0.clone())
    }

    pub fn global_by_name(&self, name: &str) -> Option<(u32, String)> {
        self.globals.get(name).and_then(|v| {
            match v.dtype {
                ObjectType::Function(ref func) => Some(func.clone()),
                _ => None,
            }
        })
    }
   
    fn preinit(&mut self) -> Result <(), String> {
        self.globals.insert(
            PUBKEY_NAME.to_string(), 
            Object::new(PUBKEY_NAME.to_string(), DATA_TYPENAME)
        );
        match self.globals.get_mut(PUBKEY_NAME).unwrap().dtype {
            ObjectType::Data {addr: _, ref mut values, ref mut persistent} => {
                *persistent = true;
                values.push(DataValue::Empty);
            },
            _ => ()
        };

        self.globals.insert(
            SCI_NAME.to_string(), 
            Object::new(SCI_NAME.to_string(), DATA_TYPENAME)
        );
        match self.globals.get_mut(SCI_NAME).unwrap().dtype {
            ObjectType::Data {addr: _, ref mut values, ref mut persistent} => {
                *persistent = false;
                values.push(DataValue::Empty);
            },
            _ => ()
        };
        ok!()
    }

    fn update_predefined(&mut self) {
        if let ObjectType::Data { ref mut addr, values: _, persistent: _ } 
            = self.globals.get_mut(SCI_NAME).unwrap().dtype {
            *addr = self.globl_base;
        }

        if let ObjectType::Data { ref mut addr, values: _, persistent: _ } 
            = self.globals.get_mut(PUBKEY_NAME).unwrap().dtype {
            *addr = self.persistent_base;
        }
    }

    pub fn parse_code<R: BufRead>(&mut self, reader: &mut R, first_pass: bool) -> Result<(), String> {
        let globl_regex = Regex::new(PATTERN_GLOBL).unwrap();
        let internal_regex = Regex::new(PATTERN_INTERNAL).unwrap();
        let selector_regex = Regex::new(PATTERN_SELECTOR).unwrap();
        let data_regex = Regex::new(PATTERN_DATA).unwrap();
        let label_regex = Regex::new(PATTERN_LABEL).unwrap();
        let dotted_regex = Regex::new(PATTERN_PARAM).unwrap();
        let alias_regex = Regex::new(PATTERN_ALIAS).unwrap();
        let type_regex = Regex::new(PATTERN_TYPE).unwrap();
        let size_regex = Regex::new(PATTERN_SIZE).unwrap();
        let base_glbl_regex = Regex::new(PATTERN_GLBLBASE).unwrap();
        let base_pers_regex = Regex::new(PATTERN_PERSBASE).unwrap();
        let ignored_regex = Regex::new(PATTERN_IGNORED).unwrap();

        let mut section_name: String = String::new();
        let mut obj_body: String = "".to_owned();
        let mut obj_name: String = "".to_owned();
        let mut lnum = 0;
        let mut l = String::new();
              
        self.globl_ptr = self.globl_base + OFFSET_GLOBL_DATA;
        self.persistent_ptr = self.persistent_base + OFFSET_PERS_DATA;

        while reader.read_line(&mut l)
            .map_err(|_| "error while reading line")? != 0 {
            lnum += 1;
            if ignored_regex.is_match(&l) {
                //ignore unused parameters
                debug!("ignored: {}", l);
            } else if base_glbl_regex.is_match(&l) {
                // .global-base
                let cap = base_glbl_regex.captures(&l).unwrap();
                let base = cap.get(1).map(|m| m.as_str())
                    .ok_or(format!("line {}: invalid syntax for global base", lnum))?;
                self.globl_base = Ptr::from_str_radix(base, 10)
                    .map_err(|_| format!("line {}: invalid global base address", lnum))?;
                self.globl_ptr = self.globl_base + OFFSET_GLOBL_DATA;
                self.update_predefined();
            } else if base_pers_regex.is_match(&l) {
                // .persistent-base
                let cap = base_pers_regex.captures(&l).unwrap();
                let base = cap.get(1).map(|m| m.as_str())
                    .ok_or(format!("line {}: invalid syntax for persistent base", lnum))?;
                self.persistent_base = Ptr::from_str_radix(base, 10)
                    .map_err(|_| format!("line {}: invalid persistent base address", lnum))?;
                self.persistent_ptr = self.persistent_base + OFFSET_PERS_DATA;
                self.update_predefined();
            } else if type_regex.is_match(&l) {
                // .type x, @...
                //it's a mark for begining of a new object (func or data)
                self.update(&section_name, &obj_name, &obj_body, first_pass)
                    .map_err(|e| format!("line {}: {}", lnum, e))?;
                section_name = GLOBL.to_owned();
                obj_body = "".to_owned(); 
                let cap = type_regex.captures(&l).unwrap();
                obj_name = cap.get(1).unwrap().as_str().to_owned();
                let type_name = cap.get(2).ok_or(format!("line {}: .type option is invalid", lnum))?.as_str();
                let obj = self.globals.entry(obj_name.clone()).or_insert(Object::new(obj_name.clone(), &type_name));
                obj.dtype = ObjectType::from(type_name);
            } else if size_regex.is_match(&l) {
                // .size x, val
                let cap = size_regex.captures(&l).unwrap();
                let name = cap.get(1).unwrap().as_str().to_owned();
                let size_str = cap.get(2).ok_or(format!("line {}: .size option is invalid", lnum))?.as_str();
                let item_ref = self.globals.entry(name.clone()).or_insert(Object::new(name, ""));
                item_ref.size = usize::from_str_radix(size_str, 10).unwrap_or(0);
            } else if globl_regex.is_match(&l) { 
                // .globl x
                let cap = globl_regex.captures(&l).unwrap();
                let name = cap.get(1).unwrap().as_str().to_owned();
                self.globals.entry(name.clone()).or_insert(Object::new(name.clone(), ""));
            } else if data_regex.is_match(&l) {
                // .data
                //ignore, not used
            } else if selector_regex.is_match(&l) {                
                // .selector
                self.update(&section_name, &obj_name, &obj_body, first_pass)?;
                if first_pass { 
                    section_name.clear();
                } else {
                    section_name = SELECTOR.to_owned();
                }
                obj_name = "".to_owned();
                obj_body = "".to_owned();
            } else if internal_regex.is_match(&l) {
                // .internal
                self.update(&section_name, &obj_name, &obj_body, first_pass)
                    .map_err(|e| format!("line {}: {}", lnum, e))?;
                section_name = INTERNAL.to_owned();
                obj_body = "".to_owned();
                obj_name = internal_regex.captures(&l).unwrap().get(1).unwrap().as_str().to_owned();
            } else if alias_regex.is_match(&l) {
                // .internal-alias
                let cap = alias_regex.captures(&l).unwrap();
                self.aliases.insert(
                    cap.get(1).unwrap().as_str().to_owned(), 
                    i32::from_str_radix(cap.get(2).unwrap().as_str(), 10)
                        .map_err(|_| format!("line: '{}': failed to parse id", lnum))?, 
                );                
            } else if label_regex.is_match(&l) { 
                //TODO: for goto
            } else if dotted_regex.is_match(&l) {
                // .param [value]
                let cap = dotted_regex.captures(&l).unwrap();
                let param = cap.get(1).unwrap().as_str();
                match param {
                    "byte" | "long" | "short" | "quad" | "comm" | "bss" => obj_body.push_str(&l),
                    _ => Err(format!("line {}: invalid param \"{}\":{}", lnum, param, l))?,
                };
            } else {
                let l_with_numbers = if first_pass { l.to_owned() } else { self.replace_labels(&l) };
                obj_body.push_str(&l_with_numbers);
            }
            l.clear();
        }

        self.update(&section_name, &obj_name, &obj_body, first_pass)?;
        ok!()
    }

    fn update(&mut self, section: &str, func: &str, body: &str, first_pass: bool) -> Result<(), String> {
        match section {
            SELECTOR => {
                if self.entry_point.is_empty() {
                    self.entry_point = body.trim_end().to_string();
                } else {
                    return Err("Another selector found".to_string());
                }
            },
            GLOBL => {
               let abi = self.abi.clone();
               let item = self.globals.get_mut(func).unwrap();
                match &mut item.dtype {
                    ObjectType::Function(fparams) => {
                        let func_id = gen_abi_id(abi, func);
                        fparams.0 = func_id;
                        fparams.1 = body.trim_end().to_string();
                        let prev = self.xrefs.insert(func.to_string(), func_id);
                        if first_pass && prev.is_some() {
                            Err(format!(
                                "global function with id = {:x} and name \"{}\" already exist", 
                                func_id,
                                func
                            ))?;
                        }
                    },
                    ObjectType::Data { addr, ref mut values, persistent } => {
                        if func.ends_with(PERSISTENT_DATA_SUFFIX) {
                            *persistent = true;
                        }
                        Self::update_data(body, func, &mut item.size, values)?;
                        let offset = (values.len() as Ptr) * WORD_SIZE;
                        if *persistent { 
                            *addr = self.persistent_ptr;
                            self.persistent_ptr += offset;
                        } else { 
                            *addr = self.globl_ptr;
                            self.globl_ptr += offset;
                        }
                    },
                    ObjectType::None => Err(format!("The type of global object {} is unknown. Use .type {}, xxx", func, func))?,
                };
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

    fn update_data(
        body: &str, 
        name: &str, 
        item_size: &mut usize, 
        values: &mut Vec<DataValue>,
    ) -> Result<(), String> {
        lazy_static! {
            static ref PARAM_RE: Regex = Regex::new(PATTERN_PARAM).unwrap();
            static ref COMM_RE:  Regex = Regex::new(PATTERN_COMM).unwrap();
        }
        for param in body.lines() {
            if let Some(cap) = COMM_RE.captures(param) {
                let size_bytes = i64::from_str_radix(
                    cap.get(2).unwrap().as_str(), 
                    10,
                ).map_err(|_| "invalid \".comm\": invalid size".to_string())?;
                let align = i64::from_str_radix(
                    cap.get(3).unwrap().as_str(),
                    10,
                ).map_err(|_| "\".comm\": invalid align".to_string())?;

                if size_bytes <= 0  {
                    Err("\".comm\": invalid size".to_string())?;
                }
                if (align <= 0) || (align % WORD_SIZE != 0) {
                    Err("\".comm\": invalid align".to_string())?;
                }
                let value_len = (size_bytes + (align - 1)) & !(align - 1);

                for _i in 0..(value_len / WORD_SIZE) {
                    values.push(DataValue::Number((IntegerData::zero(), WORD_SIZE as usize)));
                }
                *item_size = 0;
            } else if param.trim() == ".bss" {
                //ignore this directive
            } else if let Some(cap) = PARAM_RE.captures(param) {
                let pname = cap.get(1).unwrap().as_str();
                let value_len = match pname {
                    "byte"  => 1,
                    "long"  => 4,
                    "short" => 2,
                    "quad"  => 8,
                    _ => Err(format!("invalid parameter: \"{}\":", param))?,
                };
                if *item_size < value_len {
                    Err(format!("global object {} has invalid .size value: too small", name))?;
                }
                *item_size -= value_len;
                let value = cap.get(2).map_or("", |m| m.as_str()).trim();
                values.push(DataValue::Number((
                    IntegerData::from_str_radix(value, 10)
                        .map_err(|_| format!("parameter \"{}\" has invalid value \"{}\"", pname, value))?,
                    value_len,
                )));
            }
            //.bss can be here - just ignore.
        }
        if *item_size > 0 {
            Err(format!("global object {} has invalid \".size\" value: real size = {}", name, *item_size))?;
        }
        ok!()
    }

    fn build_data(&self) -> SliceData {
        let filter = |is_persistent: bool| self.globals.iter().filter_map(move |item| {
            match &item.1.dtype {
                ObjectType::Data { addr, values, persistent } => 
                    if *persistent == is_persistent { Some((addr, values)) } else { None },
                _ => None,
            }
        });
        let globl_data_vec: Vec<(&Ptr, &Vec<DataValue>)> = filter(false).collect();
        let pers_data_vec: Vec<(&Ptr, &Vec<DataValue>)> = filter(true).collect();

        let build_dict = |data_vec: &Vec<(&Ptr, &Vec<DataValue>)>| {
            let mut dict = HashmapE::with_bit_len(64);
            for item in data_vec {
                let mut ptr = item.0.clone();
                for subitem in item.1 {
                    dict.set(ptr_to_builder(ptr).unwrap().into(), subitem.write().into()).unwrap();
                    ptr += WORD_SIZE;
                }
            }
            dict
        };
        
        let globl_dict = build_dict(&globl_data_vec);
        let mut pers_dict = build_dict(&pers_data_vec);

        pers_dict.set(
            ptr_to_builder(self.persistent_base + 8).unwrap().into(), 
            globl_dict.get_data(),
        ).unwrap();

        pers_dict.get_data()
    }

    fn replace_labels(&mut self, line: &str) -> String {
        resolve_name(line, |name| self.xrefs.get(name).map(|id| id.clone()))
        .or_else(|_| resolve_name(line, |name| self.intrefs.get(name).map(|id| id.clone())))
        .or_else(|_| resolve_name(line, |name| self.globals.get(name).and_then(|obj| {
            match &obj.dtype {
                ObjectType::Data { addr, values: _, persistent: _ } => Some(addr.clone()),
                _ => None,
            }           
        })))
        .or_else(|_| resolve_name(line, |name| {
            match name {
                "global-base" => Some(self.globl_base.clone()),
                "persistent-base" => Some(self.persistent_base.clone()),
                _ => None,
            }
         }))
        .unwrap_or(line.to_string())
    }

    pub fn debug_print(&self) {
        let line = "--------------------------";
        println!("Entry point:\n{}\n{}\n{}", line, self.entry(), line);
        println!("General-purpose functions:\n{}", line);
        for (k, v) in &self.xrefs {
            println! ("Function {:30}: id={:08X}", k, v);
        }
        for (k, v) in self.globals() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tvm::test_framework::*;
    use tvm::stack::*;

    #[test]
    fn test_parser_testlib() {
        let mut parser = ParseEngine::new();
        let pbank_file = File::open("./tests/pbank.s").unwrap();
        let test_file = File::open("./tests/test.tvm").unwrap();
        assert_eq!(parser.parse(pbank_file, vec![test_file], None), ok!());  
        parser.debug_print();

        test_case(&format!("
        ;s0 - persistent data dictionary

        ;read public key from persistent_base index,
        ;it must be empty slice
            PUSHINT {base}
            DUP
            PUSH s2
            PUSHINT 64
            DICTIGET        
            THROWIFNOT 100
            SEMPTY
            THROWIFNOT 100
     
        ;get base+8 value from the dict - it's a global data dictionary
            ADDCONST {offset}
            DUP
            PUSH s2
            PUSHINT 64
            DICTIGET
            THROWIFNOT 100
            
        ;read 4 integers starting with address 8 from global dict
            DUP
            PUSHINT 8
            SWAP
            PUSHINT 64
            DICTIGET
            THROWIFNOT 100
            LDI 256
            ENDS
            SWAP

            PUSHINT 16
            OVER
            PUSHINT 64
            DICTIGET
            THROWIFNOT 100
            LDI 256
            ENDS
            SWAP

            PUSHINT 24
            OVER
            PUSHINT 64
            DICTIGET
            THROWIFNOT 100
            LDI 256
            ENDS
            SWAP
            
            PUSHINT 32
            OVER
            PUSHINT 64
            DICTIGET
            THROWIFNOT 100
            LDI 256
            ENDS
            NIP
            
        ;read integer with address persistent_base+16 from persistent dict
            PUSH s4
            ADDCONST {offset}
            PUSH s6
            PUSHINT 64
            DICTIGET
            THROWIFNOT 100
            LDI 256
            ENDS

            BLKSWAP 2, 5
            BLKDROP 2
        ", 
        base = 1000000,
        offset = WORD_SIZE,
        ))
        .with_stack(
            Stack::new()
                .push(StackItem::Slice(parser.data().into()))
                .clone()
        )
        .expect_stack(
            Stack::new()
                .push(int!(1))
                .push(int!(2))
                .push(int!(3))
                .push(int!(4))
                .push(int!(127))
        );
    }

    #[test]
    fn test_parser_stdlib() {
        let mut parser = ParseEngine::new();
        let pbank_file = File::open("./tests/pbank.s").unwrap();
        let test_file = File::open("./stdlib.tvm").unwrap();
        assert_eq!(parser.parse(pbank_file, vec![test_file], None), ok!());
    }    

    #[test]
    fn test_parser_var_without_globl() {
        let mut parser = ParseEngine::new();
        let source_file = File::open("./tests/local_global_var.code").unwrap();
        let lib_file = File::open("./stdlib.tvm").unwrap();
        assert_eq!(parser.parse(source_file, vec![lib_file], None), ok!());
    }   

    #[test]
    fn test_parser_var_with_comm() {
        let mut parser = ParseEngine::new();
        let source_file = File::open("./tests/comm_test1.s").unwrap();
        let lib_file = File::open("./stdlib.tvm").unwrap();
        assert_eq!(parser.parse(source_file, vec![lib_file], None), ok!());
    }

    #[test]
    fn test_parser_bss() {
        let mut parser = ParseEngine::new();
        let source = File::open("./tests/bss_test1.s").unwrap();
        let lib = File::open("./stdlib.tvm").unwrap();
        assert_eq!(parser.parse(source, vec![lib], None), ok!());
    }     
}