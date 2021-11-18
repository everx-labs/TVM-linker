/*
 * Copyright 2018-2019 TON DEV SOLUTIONS LTD.
 *
 * Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
 * this file except in compliance with the License.
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific TON DEV software governing permissions and
 * limitations under the License.
 */
use abi::{gen_abi_id, load_abi_contract};
use abi_json::Contract;
use regex::Regex;
use resolver::resolve_name;
use std::collections::{HashSet, HashMap};
use std::io::{BufRead, BufReader};
use std::fs::File;
use std::path::Path;
use ton_types::{BuilderData, IBitstring, SliceData, Cell};
use ton_types::dictionary::{HashmapE, HashmapType};
use ton_vm::stack::integer::{IntegerData, serialization::{Encoding, SignedIntegerBigEndianEncoding}};
use ton_vm::stack::serialization::Serializer;
use ton_labs_assembler::{DbgPos, Line, Lines, lines_to_string};

pub type Ptr = i64;

pub struct ParseEngineResults {
    engine: ParseEngine,
}

impl ParseEngineResults {
    pub fn new(parser: ParseEngine) -> Self {
        ParseEngineResults {
            engine: parser
        }
    }
    pub fn entry(&self) -> Lines {
        self.engine.entry()
    }
    pub fn publics(&self) -> HashMap<u32, Lines> {
        self.engine.publics()
    }
    pub fn privates(&self) -> HashMap<u32, Lines> {
        self.engine.privates()
    }
    pub fn internals(&self) -> HashMap<i32, Lines> {
        self.engine.internals()
    }
    pub fn global_name(&self, id: u32) -> Option<String> {
        self.engine.global_name(id)
    }
    pub fn internal_name(&self, id: i32) -> Option<String> {
        self.engine.internal_name(id)
    }
    pub fn global_by_name(&self, name: &str) -> Option<(u32, Lines)> {
        self.engine.global_by_name(name)
    }
    pub fn persistent_data(&self) -> (i64, Option<Cell>) {
        (self.engine.persistent_base, self.engine.data())
    }
    pub fn debug_print(&self) {
        self.engine.debug_print()
    }
    pub fn version(&self) -> Option<String> {
        self.engine.version()
    }
    pub fn save_my_code(&self) -> bool {
        self.engine.save_my_code()
    }
}

pub fn ptr_to_builder(n: Ptr) -> Result<BuilderData, String> {
    let mut b = BuilderData::new();
    b.append_i64(n).map_err(|_| format!("failed to serialize an i64 to builder"))?;
    Ok(b)
}

#[derive(Clone)]
struct Func {
    pub id: u32,
    pub body: Lines,
    pub calls: Vec<u32>,
}

impl Func {
    pub fn new() -> Self {
        Func { id: 0, body: vec![], calls: vec![] }
    }
}

struct Data {
    pub addr: Ptr,
    pub values: Vec<DataValue>,
    pub persistent: bool,
}

enum ObjectType {
    None,
    Function(Func),
    Data(Data),
}

enum FunctionId {
    Name(String),
    Id(i32)
}

impl From<&str> for ObjectType {
    fn from(stype: &str) -> ObjectType {
        match stype {
            "function" => ObjectType::Function(Func { id: 0, body: vec![], calls: vec![] }),
            "object" => ObjectType::Data(Data { addr: 0, values: vec![], persistent: false }),
            _ => ObjectType::None,
        }
    }
}

impl ObjectType {
    pub fn is_func(&self) -> bool {
        match self {
            ObjectType::Function(_) => true,
            _ => false,
        }
    }

    pub fn func_mut(&mut self) -> Option<&mut Func> {
        match self {
            ObjectType::Function(params) => Some(params),
            _ => None,
        }
    }

    pub fn func(&self) -> Option<&Func> {
        match self {
            ObjectType::Function(params) => Some(params),
            _ => None,
        }
    }

    pub fn data_mut(&mut self) -> Option<&mut Data> {
        match self {
            ObjectType::Data(params) => Some(params),
            _ => None,
        }
    }

    pub fn data(&self) -> Option<&Data> {
        match self {
            ObjectType::Data(params) => Some(params),
            _ => None,
        }
    }
}

const WORD_SIZE: Ptr = 1;
const OFFSET_GLOBL_DATA: Ptr = 8;
const OFFSET_PERS_DATA: Ptr = 16;

#[allow(dead_code)]
enum DataValue {
    Empty,
    Number((IntegerData, usize)),
    Slice(SliceData),
}

impl std::fmt::Display for DataValue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DataValue::Number(ref integer) => {
                write!(f, "(int {})", integer.0)
            },
            DataValue::Slice(ref _slice) => { write!(f, "(slice)") },
            DataValue::Empty => { write!(f, "(empty)") },
        }
    }
}

impl DataValue {
    pub fn write(&self) -> BuilderData {
        let mut b = BuilderData::new();
        match self {
            DataValue::Number(ref integer) => {
                let encoding = SignedIntegerBigEndianEncoding::new(257);
                let bitstring = encoding.try_serialize(&integer.0).unwrap();
                b.append_builder(&bitstring).unwrap();
                b
            },
            DataValue::Slice(ref slice) => { b.checked_append_references_and_data(slice).unwrap(); b },
            DataValue::Empty => b,
        }
    }
    pub fn size(&self) -> Ptr {
        match self {
            DataValue::Number(ref integer) => integer.1 as Ptr * WORD_SIZE,
            DataValue::Slice(ref _slice) => WORD_SIZE,
            DataValue::Empty => WORD_SIZE,
        }
    }
}

struct Object {
    pub name: String,
    pub size: usize,
    pub public: bool,
    pub dtype: ObjectType,
}

impl Object {
    pub fn new(name: String, stype: &str) -> Self {
        Object {
            name,
            size: 0,
            public: false,
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
    /// .internal function references (name -> id)
    intrefs: HashMap<String, i32>,
    /// .internal functions bodies (id -> body)
    internals: HashMap<i32, Func>,
    /// map of aliases for function names
    aliases: HashMap<String, i32>,
    /// .globl functions references (name -> id)
    xrefs: HashMap<String, u32>,
    /// map of .global objects: functions (private and public)
    /// or variables (name -> object)
    globals: HashMap<String, Object>,
    /// ID for next private global function
    next_private_globl_funcid: u32,
    /// map of macros
    macros: HashMap<String, Lines>,
    /// selector code
    entry_point: Lines,
    /// starting key for objects in global memory dictionary
    globl_base: Ptr,
    /// key for next object in global memory dictionary
    globl_ptr: Ptr,
    persistent_base: Ptr,
    persistent_ptr: Ptr,
    /// Contract ABI info, used for correct function id calculation
    abi: Option<Contract>,
    /// Contract version
    version: Option<String>,
    /// Selector variant
    save_my_code: bool,
}

const PATTERN_GLOBL:    &'static str = r"^\s*\.globl\s+(:?[\w\.]+)";
const PATTERN_DATA:     &'static str = r"^\s*\.data";
const PATTERN_INTERNAL: &'static str = r"^\s*\.internal\s+(:\w+)";
const PATTERN_SELECTOR: &'static str = r"^\s*\.selector";
const PATTERN_ALIAS:    &'static str = r"^\s*\.internal-alias (:\w+),\s+(-?\d+)";
const PATTERN_GLBLBASE: &'static str = r"^\s*\.global-base\s+(\d+)";
const PATTERN_PERSBASE: &'static str = r"^\s*\.persistent-base\s+(\d+)";
const PATTERN_LABEL:    &'static str = r"^:?[\.\w]+:";
const PATTERN_PARAM:    &'static str = r#"^\s+\.(\w+),?\s*([a-zA-Z0-9-_\s"]+)"#;
const PATTERN_TYPE:     &'static str = r"^\s*\.type\s+(:?[\w\.]+),\s*@([a-zA-Z]+)";
const PATTERN_PUBLIC:   &'static str = r"^\s*\.public\s+([\w\.]+)";
const PATTERN_SIZE:     &'static str = r"^\s*\.size\s+([\w\.]+),\s*([\.\w]+)";
const PATTERN_COMM:     &'static str = r"^\s*\.comm\s+([\w\.]+),\s*(\d+),\s*(\d+)";
const PATTERN_ASCIZ:    &'static str = r#"^\s*\.asciz\s+"(.+)""#;
const PATTERN_MACRO:    &'static str = r"^\s*\.macro\s+([\w\.:]+)";
const PATTERN_IGNORED:  &'static str = r"^\s+\.(p2align|align|text|file|ident|section)";
const PATTERN_LOC:      &'static str = r"^\s*\.loc\s+(.+),\s+(\d+)\n$";
const PATTERN_VERSION:  &'static str = r"^\s*\.version\s+(.+)";
const PATTERN_PRAGMA:   &'static str = r"^\s*\.pragma\s+(.+)";

const GLOBL:            &'static str = ".globl";
const INTERNAL:         &'static str = ".internal";
const MACROS:           &'static str = ".macro";
const SELECTOR:         &'static str = ".selector";

const DATA_TYPENAME:    &'static str = "object";

const PERSISTENT_DATA_SUFFIX: &'static str = "_persistent";

const PUBKEY_NAME:      &'static str = "tvm_public_key";
const SCI_NAME:         &'static str = "tvm_contract_info";

impl ParseEngine {

    pub fn new(sources: Vec<&Path>, abi_json: Option<String>) -> Result<Self, String> {
        let mut engine = ParseEngine {
            xrefs:      HashMap::new(),
            intrefs:    HashMap::new(),
            aliases:    HashMap::new(),
            globals:    HashMap::new(),
            next_private_globl_funcid: 0,
            internals:  HashMap::new(),
            macros:     HashMap::new(),
            entry_point: vec![],
            globl_base:      0,
            globl_ptr:       0,
            persistent_base: 0,
            persistent_ptr:  0,
            abi:             None,
            version:         None,
            save_my_code:    false,
        };
        engine.parse(sources, abi_json)?;
        Ok(engine)
    }

    fn parse(&mut self, sources: Vec<&Path>, abi_json: Option<String>) -> Result<(), String> {
        if let Some(s) = abi_json {
            self.abi = Some(load_abi_contract(&s)?);
        }

        self.preinit()?;

        for source in &sources {
            self.parse_code(source)?;
        }

        self.replace_all_labels()?;

        self.drop_unused_objects();
        Ok(())
    }

    fn data(&self) -> Option<Cell> {
        self.build_data()
    }

    fn entry(&self) -> Lines {
        self.entry_point.clone()
    }

    fn internals(&self) -> HashMap<i32, Lines> {
        let mut funcs = HashMap::new();
        self.internals.iter().for_each(|x| {
            funcs.insert(x.0.clone(), x.1.body.clone());
        });
        funcs
    }

    fn internal_name(&self, id: i32) -> Option<String> {
        self.intrefs.iter().find(|i| *i.1 == id).map(|i| i.0.clone())
    }

    #[allow(dead_code)]
    fn internal_by_name(&self, name: &str) -> Option<(i32, Lines)> {
        let id = self.intrefs.get(name)?;
        let body = self.internals.get(id).map(|f| f.body.to_owned())?;
        Some((*id, body))
    }

    fn publics(&self) -> HashMap<u32, Lines> {
        self.globals(true)
    }

    fn privates(&self) -> HashMap<u32, Lines> {
        self.globals(false)
    }

    fn globals(&self, public: bool) -> HashMap<u32, Lines> {
        let mut funcs = HashMap::new();
        let iter = self.globals.iter().filter_map(|item| {
            item.1.dtype.func().and_then(|i| {
                if public == item.1.public {
                    Some(i)
                } else {
                    None
                }
            })
        });
        for i in iter {
            funcs.insert(i.id, i.body.clone());
        }
        funcs
    }

    fn global_name(&self, id: u32) -> Option<String> {
        self.globals.iter().find(|item| {
            if let Some(func) = item.1.dtype.func() {
                func.id == id
            } else {
                false
            }
        })
        .map(|i| i.0.clone())
    }

    fn global_by_name(&self, name: &str) -> Option<(u32, Lines)> {
        self.globals.get(name).and_then(|v| {
            v.dtype.func().and_then(|func| Some((func.id.clone(), func.body.clone()) ))
        })
    }

    fn preinit(&mut self) -> Result <(), String> {
        self.globals.insert(
            PUBKEY_NAME.to_string(),
            Object::new(PUBKEY_NAME.to_string(), DATA_TYPENAME)
        );
        self.globals.get_mut(PUBKEY_NAME)
            .unwrap()
            .dtype
            .data_mut()
            .and_then(|data| {
                data.persistent = true;
                data.values.push(DataValue::Empty);
                Some(data)
            });

        self.globals.insert(
            SCI_NAME.to_string(),
            Object::new(SCI_NAME.to_string(), DATA_TYPENAME)
        );
        self.globals.get_mut(SCI_NAME)
            .unwrap()
            .dtype
            .data_mut()
            .and_then(|data| {
                data.persistent = false;
                data.values.push(DataValue::Empty);
                Some(data)
            });
        Ok(())
    }

    fn update_predefined(&mut self) {
        let data = self.globals.get_mut(SCI_NAME)
            .unwrap()
            .dtype
            .data_mut()
            .unwrap();
        data.addr = self.globl_base;

        let data = self.globals.get_mut(PUBKEY_NAME)
            .unwrap()
            .dtype
            .data_mut()
            .unwrap();
        data.addr = self.persistent_base;
    }

    fn version(&self) -> Option<String> {
        self.version.clone()
    }

    fn save_my_code(&self) -> bool {
        self.save_my_code
    }

    fn parse_code(&mut self, path: &Path) -> Result<(), String> {
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
        let public_regex = Regex::new(PATTERN_PUBLIC).unwrap();
        let macro_regex = Regex::new(PATTERN_MACRO).unwrap();
        let loc_regex = Regex::new(PATTERN_LOC).unwrap();
        let version_regex = Regex::new(PATTERN_VERSION).unwrap();
        let pragma_regex = Regex::new(PATTERN_PRAGMA).unwrap();

        let mut section_name: String = String::new();
        let mut obj_body: Lines = vec![];
        let mut obj_name: String = "".to_owned();
        let mut lnum = 0;
        let mut l = String::new();

        self.globl_ptr = self.globl_base + OFFSET_GLOBL_DATA;
        self.persistent_ptr = self.persistent_base + OFFSET_PERS_DATA;

        let filename = path.file_name().unwrap().to_str().unwrap().to_string();
        let file = File::open(path).map_err(|e| format!("Can't open file {}: {}", filename, e))?;
        let mut reader = BufReader::new(file);
        let mut source_pos: Option<DbgPos> = None;

        while reader.read_line(&mut l)
            .map_err(|_| format!("error while reading line (file: {})", filename))? != 0 {
            lnum += 1;

            l = l.replace("\r", "");
            if !l.ends_with('\n') {
                l += "\n";
            }

            let pos = match source_pos.clone() {
                None => DbgPos { filename: filename.clone(), line: lnum, line_code: lnum },
                Some(pos) => pos
            };
            if ignored_regex.is_match(&l) {
                //ignore unused parameters
                debug!("ignored: {}", l);
            } else if version_regex.is_match(&l) {
                let cap = version_regex.captures(&l).unwrap();
                self.version = Some(cap.get(1).unwrap().as_str().to_owned());
            } else if pragma_regex.is_match(&l) {
                let cap = pragma_regex.captures(&l).unwrap();
                match cap.get(1) {
                    Some(m) => if m.as_str() == "selector-save-my-code" {
                        self.save_my_code = true
                    },
                    None => {}
                }
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
                self.update(&section_name, &obj_name, &obj_body)
                    .map_err(|e| format!("line {}: {}", lnum, e))?;
                section_name = GLOBL.to_owned();
                obj_body = vec![];
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
            } else if public_regex.is_match(&l) {
                // .public x
                let cap = public_regex.captures(&l).unwrap();
                let name = cap.get(1).unwrap().as_str();
                self.globals.get_mut(name).and_then(|obj| {obj.public = true; Some(obj)});
            } else if globl_regex.is_match(&l) {
                // .globl x
                let cap = globl_regex.captures(&l).unwrap();
                let name = cap.get(1).unwrap().as_str().to_owned();
                self.globals.entry(name.clone()).or_insert(Object::new(name.clone(), ""));
            } else if macro_regex.is_match(&l) {
                // .macro x
                self.update(&section_name, &obj_name, &obj_body)
                    .map_err(|e| format!("line {}: {}", lnum, e))?;
                section_name = MACROS.to_owned();
                obj_body = vec![];
                obj_name = macro_regex.captures(&l).unwrap().get(1).unwrap().as_str().to_owned();
            } else if data_regex.is_match(&l) {
                // .data
                //ignore, not used
            } else if selector_regex.is_match(&l) {
                // .selector
                self.update(&section_name, &obj_name, &obj_body)?;
                section_name = SELECTOR.to_owned();
                obj_name = "".to_owned();
                obj_body = vec![];
            } else if internal_regex.is_match(&l) {
                // .internal
                self.update(&section_name, &obj_name, &obj_body)
                    .map_err(|e| format!("line {}: {}", lnum, e))?;
                section_name = INTERNAL.to_owned();
                obj_body = vec![];
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
                // ignore labels
            } else if loc_regex.is_match(&l) {
                let cap = loc_regex.captures(&l).unwrap();
                let filename = String::from(cap.get(1).unwrap().as_str());
                let line = cap.get(2).unwrap().as_str().parse::<usize>().unwrap();
                if line == 0 { // special value for resetting current source pos
                    source_pos = None;
                } else {
                    source_pos = Some(DbgPos { filename, line, line_code: lnum });
                }
            } else if dotted_regex.is_match(&l) {
                // .param [value]
                let cap = dotted_regex.captures(&l).unwrap();
                let param = cap.get(1).unwrap().as_str();
                match param {
                    "blob" | "cell" | "byte" | "long" | "short" | "quad" | "comm" | "bss" | "asciz" | "compute" => {
                        obj_body.push(Line { text: l.clone(), pos })
                    },
                    _ => Err(format!("line {}: invalid param \"{}\":{}", lnum, param, l))?,
                };
            } else {
                obj_body.push(Line { text: l.clone(), pos });
            }
            l.clear();
        }

        if section_name.is_empty() {
            return Err("input file has no assembler definitions".to_string());
        }

        self.update(&section_name, &obj_name, &obj_body)
            .map_err(|e| format!("line {}: {}", lnum, e))?;
        Ok(())
    }

    fn replace_all_labels(&mut self) -> Result<(), String> {
        let mut iter = 0;
        loop {
            iter += 1;
            if iter >= 50 {
                return Err("There are recursive macros or level of nested macros >= 50".to_string());
            }
            match self.try_replace_labels() {
                Ok(do_continue) if do_continue => {
                    continue;
                }
                Ok(_) => {
                    return Ok(());
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }

    fn replace_labels_in_body(&mut self, lines: Vec<Line>, obj_name: FunctionId) -> Result<Vec<Line>, String> {
        let mut new_lines = vec![];
        for line in lines {
            let mut resolved =
                self.replace_labels(&line, &obj_name)
                    .map_err(|e| format!("line {}: cannot resolve label: {}", line.pos.line, e))?;
            new_lines.append(&mut resolved);
        }
        Ok(new_lines)
    }

    // return true, if at least one label was replaced
    fn try_replace_labels(&mut self) -> Result<bool, String> {
        let mut did_some = false;

        let names = self.globals.keys().map(|k| k.clone()).collect::<Vec<_>>();
        for name in &names {
            if let ObjectType::Function(f) = &self.globals.get(name).unwrap().dtype {
                let lines = f.body.clone();
                let obj_name = self.globals.get(name).unwrap().name.clone();
                let new_lines = self.replace_labels_in_body(lines, FunctionId::Name(obj_name))?;

                let body = &mut self.globals.get_mut(name).unwrap().dtype.func_mut().unwrap().body;
                did_some = did_some || *body != new_lines;
                *body = new_lines;
            }
        }

        let ids = self.internals.keys().map(|x| *x).collect::<Vec<_>>();
        for id in &ids {
            let lines = self.internals.get(id).unwrap().body.clone();
            let new_lines = self.replace_labels_in_body(lines, FunctionId::Id(*id))?;
            let body = &mut self.internals.get_mut(id).unwrap().body;
            did_some = did_some || *body != new_lines;
            *body = new_lines;
        }

        Ok(did_some)
    }

    fn create_function_id(&mut self, func: &str) -> u32 {
        let is_public = self.globals.get(func).unwrap().public;
        if is_public {
            gen_abi_id(self.abi.clone(), func)
        } else {
            self.next_private_globl_funcid += 1;
            self.next_private_globl_funcid
        }
    }

    fn is_public(&self, globl_name: &str) -> bool {
        self.abi.as_ref()
            .map(|abi| {
                abi.functions().get(globl_name).is_some()
                || abi.events().get(globl_name).is_some()
            })
            .unwrap_or(false)
    }

    fn update(&mut self, section: &str, name: &str, body: &Lines) -> Result<(), String> {
        match section {
            SELECTOR => {
                if self.entry_point.is_empty() {
                    self.entry_point = body.clone();
                } else {
                    return Err("Another selector found".to_string());
                }
            },
            GLOBL => {
                //do not reset public flag if symbol isn't included in ABI,
                //because it can be marked as public in assembly.
                if self.is_public(name) {
                    self.globals.get_mut(name).unwrap().public = true;
                }

                if self.globals.get(name).unwrap().dtype.is_func() {
                    // globl object is function
                    let func_id = self.create_function_id(name);
                    let item = self.globals.get_mut(name).unwrap();
                    let params = item.dtype.func_mut().unwrap();
                    params.id = func_id;
                    params.body = body.clone();
                    let prev = self.xrefs.insert(name.to_string(), func_id);
                    if prev.is_some() {
                        Err(format!(
                            "global function with id = {:x} and name \"{}\" already exist",
                            func_id,
                            name,
                        ))?;
                    }
                } else {
                    // globl object is data
                    let item = self.globals.get_mut(name).unwrap();
                    let data = item.dtype.data_mut().unwrap();
                    Self::update_data(body, name, &mut item.size, &mut data.values)?;
                    let offset = (data.values.len() as Ptr) * WORD_SIZE;
                    if name.ends_with(PERSISTENT_DATA_SUFFIX) {
                        data.persistent = true;
                        data.addr = self.persistent_ptr;
                        self.persistent_ptr += offset;
                    } else {
                        data.addr = self.globl_ptr;
                        self.globl_ptr += offset;
                    }
                }
            },
            INTERNAL => {
                let func_id = self.aliases.get(name).ok_or(format!("id for '{}' not found", name))?;
                self.intrefs.insert(name.to_string(), *func_id);
                let prev = self.internals.insert(*func_id,Func::new());
                if prev.is_some() {
                    Err(format!("internal function with id = {} already exist", *func_id))?;
                }
                self.internals.get_mut(func_id).unwrap().body = body.clone();
            },
            MACROS => {
                let prev = self.macros.insert(name.to_string(), body.clone());
                if prev.is_some() {
                    Err(format!("macros with name \"{}\" already exist", name))?;
                }
            },
            _ => (),
        }
        Ok(())
    }

    fn update_data(
        body: &Lines,
        name: &str,
        item_size: &mut usize,
        values: &mut Vec<DataValue>,
    ) -> Result<(), String> {
        lazy_static! {
            static ref PARAM_RE: Regex = Regex::new(PATTERN_PARAM).unwrap();
            static ref COMM_RE:  Regex = Regex::new(PATTERN_COMM).unwrap();
            static ref ASCI_RE:  Regex = Regex::new(PATTERN_ASCIZ).unwrap();
        }
        for param in body {
            let mut value_len: usize = 0;
            if let Some(cap) = COMM_RE.captures(param.text.as_str()) {
                // .comm <symbol>, <size>, <align>
                let size_bytes = usize::from_str_radix(
                    cap.get(2).unwrap().as_str(),
                    10,
                ).map_err(|_| "invalid \".comm\": invalid size".to_string())?;
                let align = usize::from_str_radix(
                    cap.get(3).unwrap().as_str(),
                    10,
                ).map_err(|_| "\".comm\": invalid align".to_string())?;

                if size_bytes == 0  {
                    Err("\".comm\": invalid size".to_string())?;
                }
                if (align == 0) || (align % WORD_SIZE as usize != 0) {
                    Err("\".comm\": invalid align".to_string())?;
                }
                value_len = (size_bytes + (align - 1)) & !(align - 1);
                for _i in 0..(value_len / WORD_SIZE as usize) {
                    values.push(DataValue::Number((IntegerData::zero(), WORD_SIZE as usize)));
                }
                *item_size = value_len;
            } else if param.text.trim() == ".bss" {
                //ignore this directive
            } else if let Some(cap) = ASCI_RE.captures(param.text.as_str()) {
                // .asciz "string"
                let mut str_bytes = cap.get(1).unwrap().as_str().as_bytes().to_vec();
                //include 1 byte for termination zero, assume that it is C string
                value_len = str_bytes.len() + 1;
                str_bytes.push(0);
                for cur_char in str_bytes {
                    values.push(DataValue::Number((IntegerData::from(cur_char).unwrap(), 1)));
                }
            } else if let Some(cap) = PARAM_RE.captures(param.text.as_str()) {
                let pname = cap.get(1).unwrap().as_str();
                value_len = match pname {
                    "byte"  => 1,
                    "long"  => 4,
                    "short" => 2,
                    "quad"  => 8,
                    _ => Err(format!("invalid parameter: \"{}\"", param.text))?,
                };
                let value = cap.get(2).map_or("", |m| m.as_str()).trim();
                values.push(DataValue::Number((
                    IntegerData::from_str_radix(value, 10)
                        .map_err(|_| format!("parameter \"{}\" has invalid value \"{}\"", pname, value))?,
                    value_len,
                )));
            }
            if *item_size < value_len {
                Err(format!("global object {} has invalid .size parameter: too small", name))?;
            }
            *item_size -= value_len;
        }
        if *item_size > 0 {
            Err(format!("global object {} has invalid \".size\" value: real size = {}", name, *item_size))?;
        }
        Ok(())
    }

    fn build_data(&self) -> Option<Cell> {
        let filter = |persistent: bool| {
            self.globals.iter().filter_map(move |item| {
                item.1.dtype.data().and_then(|data| {
                    if data.persistent == persistent {
                        Some((&data.addr, &data.values))
                    } else {
                        None
                    }
                })
            })
        };
        let globl_data_vec: Vec<(&Ptr, &Vec<DataValue>)> = filter(false).collect();
        let pers_data_vec: Vec<(&Ptr, &Vec<DataValue>)> = filter(true).collect();

        let build_dict = |data_vec: &Vec<(&Ptr, &Vec<DataValue>)>| {
            let mut dict = HashmapE::with_bit_len(64);
            for item in data_vec {
                let mut ptr = item.0.clone();
                for subitem in item.1 {
                    dict.set(ptr_to_builder(ptr).unwrap().into_cell().unwrap().into(), &subitem.write().into_cell().unwrap().into()).unwrap();
                    ptr += subitem.size();
                }
            }
            dict
        };

        let globl_dict = build_dict(&globl_data_vec);
        let mut pers_dict = build_dict(&pers_data_vec);
        let mut globl_cell = BuilderData::new();
        if let Some(cell) = globl_dict.data() {
            globl_cell.append_bit_one()
                .unwrap()
                .checked_append_reference(cell.clone())
                .unwrap();
        } else {
            globl_cell.append_bit_zero().unwrap();
        }
        pers_dict.set(
            ptr_to_builder(self.persistent_base + OFFSET_GLOBL_DATA).unwrap().into_cell().unwrap().into(),
            &globl_cell.into_cell().unwrap().into()
        ).unwrap();

        pers_dict.data().map(|cell| cell.clone())
    }

    fn cell_encode(&self, cell: &Cell, toplevel: bool) -> Lines {
        let slice = SliceData::from(cell);
        let mut lines = vec!();
        let opening = if toplevel { "{\n" } else { ".cell {\n" };
        lines.push(Line::new(opening, "", 0));
        lines.push(Line::new(format!(".blob x{}\n", slice.to_hex_string()).as_str(), "", 0));
        for i in slice.get_references() {
            let child = cell.reference(i).unwrap();
            let mut child_lines = self.cell_encode(&child, false);
            lines.append(&mut child_lines);
        }
        lines.push(Line::new("}\n", "", 0));
        lines
    }

    fn cell_compute(&self, name: &str, lines: &Lines) -> Result<Lines, String> {
        let (code, _) = ton_labs_assembler::compile_code_debuggable(lines.clone())
            .map_err(|ce| ce.to_string())?;

        let mut engine = ton_vm::executor::Engine::new().setup_with_libraries(
            code, None, None, None, vec![]);
        match engine.execute() {
            Err(e) => {
                println!("failed to compute cell: {}", e);
                return Err(name.to_string())
            }
            Ok(code) => {
                if code != 0 {
                    println!("failed to compute cell, exit code {}", code);
                    return Err(name.to_string())
                }
            }
        };

        let cell = engine.stack().get(0).as_cell().map_err(|_| name)?;
        Ok(self.cell_encode(cell, true))
    }

    fn replace_labels(&mut self, line: &Line, cur_obj_name: &FunctionId) -> Result<Lines, String> {
        lazy_static! {
            static ref COMPUTE_REGEX: Regex = Regex::new(r"^\s*\.compute\s+\$([\w\.:]+)\$").unwrap();
        }
        if COMPUTE_REGEX.is_match(&line.text) {
            let name = COMPUTE_REGEX.captures(&line.text).unwrap().get(1).unwrap().as_str();
            let lines = self.macros.get(name).ok_or(name)?;
            return self.cell_compute(name, lines)
        }
        resolve_name(line, |name| {
            self.intrefs.get(name).and_then(|id| Some(id.clone()))
        })
        .or_else(|_| resolve_name(line, |name| {
            let mut res = self.xrefs.get(name).map(|id| id.clone());
            if res.is_some(){
                let id = res.unwrap();
                self.insert_called_func(&cur_obj_name, id);
                res = Some(id);
            }
            res
        }))
        .or_else(|_| resolve_name(line, |name| {
            self.globals.get(name).and_then(|obj| {
                obj.dtype.data().and_then(|data| Some(data.addr.clone()))
            })
        }))
        .or_else(|_| resolve_name(line, |name| {
            match name {
                "global-base" => Some(self.globl_base.clone()),
                "persistent-base" => Some(self.persistent_base.clone()),
                _ => None,
            }
        }))
        .or_else(|e| {
            let mut name = String::new();
            resolve_name(line, |n| { name = n.to_string(); Some(0)}).unwrap();
            self.macros.get(&name)
                .ok_or(e)
                .map(|body| body.clone())
        })
    }

    fn insert_called_func(&mut self, from_func: &FunctionId, to_func: u32) {
        match from_func {
            FunctionId::Name(name) => {
                self.globals.get_mut(name)
                    .and_then(|obj| {
                        obj.dtype.func_mut().and_then(|f| {
                            f.calls.push(to_func);
                            Some(f)
                        })
                    });
            }
            FunctionId::Id(id) => {
                self.internals.get_mut(&id).and_then(|f| {
                    f.calls.push(to_func);
                    Some(f)
                });
            }
        }
    }

    fn drop_unused_objects(&mut self) {
        let mut ids = HashSet::new();
        let publics_iter = self.globals.iter().filter_map(|obj| {
            obj.1.dtype.func()
                .and_then(|i| if obj.1.public { Some(i) } else { None })
        });

        for func in publics_iter {
            self.enum_calling_funcs(&func, &mut ids);
        }
        for func in self.internals.iter() {
            self.enum_calling_funcs(&func.1, &mut ids);
        }

        self.globals.retain(|_k, v| {
            v.dtype.func()
                .map(|f| ids.contains(&f.id))
                .unwrap_or(true)
        });
        self.xrefs.retain(|_k, v| {
            ids.contains(&v)
        });
    }

    fn enum_calling_funcs(&self, func: &Func, ids: &mut HashSet<u32>) {
        ids.insert(func.id); // TODO there are public/private globs and internals

        for id in &func.calls {
            if ids.insert(*id) {
                let subfunc = self.globals.iter().find(|(_name, obj)| {
                    obj.dtype.func().map(|f| f.id == *id).unwrap_or(false)
                })
                    .map(|(_name, obj)| obj.dtype.func().unwrap());
                if subfunc.is_some() {
                    self.enum_calling_funcs(&subfunc.unwrap(), ids);
                }
            }
        }
    }

    fn debug_print(&self) {
        let line = "--------------------------";
        let entry = lines_to_string(&self.entry());
        println!("Entry point:\n{}\n{}\n{}", line, entry, line);
        println!("General-purpose functions:\n{}", line);

        let mut keys = self.xrefs.keys().collect::<Vec<_>>();
        keys.sort();
        for k in keys {
            println! ("Function {:30}: id={:08X} public={}",
                      k,
                      self.xrefs.get(k).unwrap(),
                      self.globals.get(k).unwrap().public);
        }
        println!("private:");
        for (k, v) in &self.privates() {
            let code = lines_to_string(&v);
            println! ("Function {:08X}\n{}\n{}\n{}", k, line, code, line);
        }
        println!("public:");
        for (k, v) in self.publics() {
            let code = lines_to_string(&v);
            println! ("Function {:08X}\n{}\n{}\n{}", k, line, code, line);
        }
        println!("{}\nInternal functions:\n{}", line, line);
        for (k, v) in &self.intrefs {
            println! ("Function {:30}: id={:08X}", k, v);
        }
        for (k, v) in &self.internals {
            let code = lines_to_string(&v.body);
            println! ("Function {:08X}\n{}\n{}\n{}", k, line, code, line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ton_vm::executor::Engine;
    use ton_labs_assembler::compile_code;
    use ton_vm::stack::{Stack, StackItem};
    use std::sync::Arc;

    #[test]
    fn test_parser_testlib() {
        let sources = vec![Path::new("./tests/test.tvm")];
        let parser = ParseEngine::new(sources, None);
        assert_eq!(parser.is_ok(), true);
        let parser = parser.unwrap();

        let mut data_dict = BuilderData::new();
        data_dict.append_bit_one().unwrap().checked_append_reference(parser.data().unwrap()).unwrap();

        let code = compile_code(&format!("
        ;s0 - persistent data dictionary
            PLDDICT
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
            PLDDICT

        ;read 4 integers starting with address 8 from global dict
            DUP
            PUSHINT 8
            SWAP
            PUSHINT 64
            DICTIGET
            THROWIFNOT 100
            PUSHINT 257 LDIX
            ENDS
            SWAP

            PUSHINT 12
            OVER
            PUSHINT 64
            DICTIGET
            THROWIFNOT 100
            PUSHINT 257 LDIX
            ENDS
            SWAP

            PUSHINT 16
            OVER
            PUSHINT 64
            DICTIGET
            THROWIFNOT 100
            PUSHINT 257 LDIX
            ENDS
            SWAP

            PUSHINT 20
            OVER
            PUSHINT 64
            DICTIGET
            THROWIFNOT 100
            PUSHINT 257 LDIX
            ENDS
            NIP

        ;read integer with address persistent_base+16 from persistent dict
            PUSH s4
            ADDCONST {offset}
            PUSH s6
            PUSHINT 64
            DICTIGET
            THROWIFNOT 100
            PUSHINT 257 LDIX
            ENDS

            BLKSWAP 2, 5
            BLKDROP 2
        ",
        base = 1000000,
        offset = OFFSET_GLOBL_DATA,
        )).expect("Couldn't compile code");

        let mut stack = Stack::new();
        stack.push(StackItem::Slice(data_dict.into_cell().unwrap().into()));

        let mut engine = Engine::new().setup_with_libraries(code, None, Some(stack), None, vec![]);
        engine.set_trace(Engine::TRACE_ALL);
        engine.execute().unwrap();

        engine.assert_stack(Stack::new()
            .push(int!(1))
            .push(int!(2))
            .push(int!(3))
            .push(int!(4))
            .push(int!(127)));
    }

    #[test]
    fn test_parser_var_without_globl() {
        let sources = vec![Path::new("./tests/test_stdlib.tvm"),
                                     Path::new("./tests/local_global_var.code")];
        let parser = ParseEngine::new(sources, None);
        assert_eq!(parser.is_ok(), true);
    }

    #[test]
    fn test_parser_var_with_comm() {
        let sources = vec![Path::new("./tests/test_stdlib.tvm"),
                                     Path::new("./tests/comm_test1.s")];
        let parser = ParseEngine::new(sources, None);
        assert_eq!(parser.is_ok(), true);
    }

    #[test]
    fn test_parser_bss() {
        let sources = vec![Path::new("./tests/test_stdlib.tvm"),
                                     Path::new("./tests/bss_test1.s")];
        let parser = ParseEngine::new(sources, None);
        assert_eq!(parser.is_ok(), true);
    }

    #[test]
    fn test_multilibs() {
        let sources = vec![Path::new("./tests/testlib1.tvm"),
                                     Path::new("./tests/testlib2.tvm"),
                                     Path::new("./tests/hello.code")];
        let parser = ParseEngine::new(sources, None);
        assert_eq!(parser.is_ok(), true);
    }

    #[test]
    fn test_external_linking() {
        let sources = vec![Path::new("./tests/test_extlink_lib.tvm"),
                                     Path::new("./tests/test_extlink_source.s")];
        let parser = ParseEngine::new(sources, None);
        assert_eq!(parser.is_ok(), true);
    }

    #[test]
    fn test_macros() {
        let sources = vec![Path::new("./tests/test_stdlib.tvm"),
                                     Path::new("./tests/test_macros.code")];
        let parser = ParseEngine::new(sources, None);
        assert_eq!(parser.is_ok(), true);
        let publics = parser.unwrap().publics();
        let body = publics.get(&0x0D6E4079).unwrap();

        assert_eq!(
            *body,
            vec![Line::new("PUSHINT 10\n", "test_macros.code", 4),
                 Line::new("DROP\n",       "test_macros.code", 5),
                 Line::new("PUSHINT 1\n",  "test_macros.code", 10),
                 Line::new("PUSHINT 2\n",  "test_macros.code", 11),
                 Line::new("ADD\n",        "test_macros.code", 12),
                 Line::new("PUSHINT 3\n",  "test_macros.code", 7),
                 Line::new("\n",           "test_macros.code", 8)]
        );
    }

    #[test]
    fn test_macros_02() {
        let sources = vec![
            Path::new("./tests/test_stdlib.tvm"),
            Path::new("./tests/test_macros_02.code")
        ];
        let parser = ParseEngine::new(sources, None).unwrap();
        let publics = parser.publics();
        let body = publics.get(&0x0D6E4079).unwrap();
        let globals = parser.globals(false);
        let internal = globals.get(&2).unwrap();

        assert_eq!(
            *body,
            vec![Line::new("PUSHINT 10\n", "test_macros_02.code", 4),
                 Line::new("DROP\n",       "test_macros_02.code", 5),
                 Line::new("PUSHINT 1\n",  "test_macros_02.code", 16),
                 Line::new("\n",           "test_macros_02.code", 17),
                 Line::new("PUSHINT 2\n",  "test_macros_02.code", 12),
                 Line::new("ADD\n",        "test_macros_02.code", 13),
                 Line::new("\n",           "test_macros_02.code", 14),
                 Line::new("PUSHINT 3\n",  "test_macros_02.code", 7),
                 Line::new("CALL 2\n",     "test_macros_02.code", 8),
                 Line::new("\n",           "test_macros_02.code", 9)]
        );
        assert_eq!(
            *internal,
            vec![Line::new("PUSHINT 1\n",  "test_macros_02.code", 16),
                 Line::new("\n",           "test_macros_02.code", 17),
                 Line::new("PUSHINT 2\n",  "test_macros_02.code", 12),
                 Line::new("ADD\n",        "test_macros_02.code", 13),
                 Line::new("\n",           "test_macros_02.code", 14)]
        );
    }

    #[test]
    fn test_compute() {
        let sources = vec![Path::new("./tests/test_stdlib.tvm"),
                                     Path::new("./tests/test_compute.code")];
        let parser = ParseEngine::new(sources, None);
        assert_eq!(parser.is_ok(), true);

        let internals = parser.unwrap().internals();
        let internal = internals.get(&-2).unwrap();

        assert_eq!(
            *internal,
            vec![
                Line::new("\n",               "test_compute.code", 3),
                Line::new("PUSHREF\n",        "test_compute.code", 4),
                Line::new("{\n",                "", 0),
                Line::new(".blob x0000006f\n",  "", 0),
                Line::new("}\n",                "", 0),
                Line::new("CTOS\n",           "test_compute.code", 6),
                Line::new("PLDU 32\n",        "test_compute.code", 7),
                Line::new("PUSHINT 111\n",    "test_compute.code", 8),
                Line::new("EQUAL\n",          "test_compute.code", 9),
                Line::new("THROWIFNOT 222\n", "test_compute.code", 10),
                Line::new("\n",               "test_compute.code", 11),
            ]
        );
    }
}
