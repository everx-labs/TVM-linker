/*
 * Copyright 2018-2022 TON DEV SOLUTIONS LTD.
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
use failure::{format_err, bail};
use regex::Regex;
use resolver::resolve_name;
use std::collections::{HashSet, HashMap};
use std::io::{BufRead, BufReader, Read};
use std::fs::File;
use std::path::Path;
use ton_types::{BuilderData, IBitstring, SliceData, Cell, Result, Status};
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

pub fn ptr_to_builder(n: Ptr) -> Result<BuilderData> {
    let mut b = BuilderData::new();
    b.append_i64(n).map_err(|e| format_err!("failed to serialize an i64 to builder: {}", e))?;
    Ok(b)
}

#[derive(Clone)]
struct InternalFunc {
    pub id: u32,
    pub body: Lines,
    // function ids that this function calls from its body
    pub called_ids: Vec<u32>,
}

impl InternalFunc {
    pub fn new() -> Self {
        InternalFunc { id: 0, body: vec![], called_ids: vec![] }
    }
}

struct Data {
    pub addr: Ptr,
    pub values: Vec<DataValue>,
    pub persistent: bool,
}

enum GloblFuncOrDataType {
    None, // TODO delete
    Function(InternalFunc),
    Data(Data),
}

enum FunctionId {
    Name(String),
    Id(i32)
}

impl From<&str> for GloblFuncOrDataType {
    fn from(stype: &str) -> GloblFuncOrDataType {
        match stype {
            "function" => GloblFuncOrDataType::Function(InternalFunc { id: 0, body: vec![], called_ids: vec![] }),
            "object" => GloblFuncOrDataType::Data(Data { addr: 0, values: vec![], persistent: false }),
            _ => GloblFuncOrDataType::None,
        }
    }
}

impl GloblFuncOrDataType {
    pub fn is_func(&self) -> bool {
        matches!(self, GloblFuncOrDataType::Function(_))
    }

    pub fn func_mut(&mut self) -> Option<&mut InternalFunc> {
        match self {
            GloblFuncOrDataType::Function(params) => Some(params),
            _ => None,
        }
    }

    pub fn func(&self) -> Option<&InternalFunc> {
        match self {
            GloblFuncOrDataType::Function(params) => Some(params),
            _ => None,
        }
    }

    pub fn data_mut(&mut self) -> Option<&mut Data> {
        match self {
            GloblFuncOrDataType::Data(params) => Some(params),
            _ => None,
        }
    }

    pub fn data(&self) -> Option<&Data> {
        match self {
            GloblFuncOrDataType::Data(params) => Some(params),
            _ => None,
        }
    }
}

const WORD_SIZE: Ptr = 1;
const OFFSET_GLOBL_DATA: Ptr = 8;
const OFFSET_PERS_DATA: Ptr = 16;

enum DataValue {
    Empty,
    Number((IntegerData, usize)),
}

impl std::fmt::Display for DataValue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DataValue::Number(ref integer) => {
                write!(f, "(int {})", integer.0)
            },
            DataValue::Empty => { write!(f, "(empty)") },
        }
    }
}

impl DataValue {
    pub fn write(&self) -> Result<BuilderData> {
        let mut b = BuilderData::new();
        Ok(match self {
            DataValue::Number(ref integer) => {
                let encoding = SignedIntegerBigEndianEncoding::new(257);
                let bitstring = encoding.try_serialize(&integer.0)?;
                b.append_builder(&bitstring)?;
                b
            },
            DataValue::Empty => b,
        })
    }
    pub fn size(&self) -> Ptr {
        match self {
            DataValue::Number(ref integer) => integer.1 as Ptr * WORD_SIZE,
            DataValue::Empty => WORD_SIZE,
        }
    }
}

struct GloblFuncOrData {
    pub name: String,
    pub size: usize,
    pub public: bool,
    pub dtype: GloblFuncOrDataType,
}

impl GloblFuncOrData {
    pub fn new(name: String, stype: &str) -> Self {
        GloblFuncOrData {
            name,
            size: 0,
            public: false,
            dtype: GloblFuncOrDataType::from(stype),
        }
    }
}

impl Default for GloblFuncOrData {
    fn default() -> Self {
        GloblFuncOrData::new(String::new(), "")
    }
}

pub struct ParseEngine {
    /// it's about .internal, e.g.
    ///.internal-alias :main_internal, 0
    // .internal :main_internal
    // ...
    /// name -> id, e.g. main_internal -> 0, main_external -> -1,
    internal_name_to_id: HashMap<String, i32>,
    /// id -> code
    internal_id_to_code: HashMap<i32, InternalFunc>,
    /// aliases for function names, e.g. main_internal -> 0, main_internal -> 0,
    internal_alias_name_to_id_: HashMap<String, i32>, // TODO delete this or internal_name_to_id

    /// it's about private/public .globl or variables, e.g.
    /// .globl  sendMessage_internal
    /// .type   sendMessage_internal, @function
    /// ...
    /// name -> id
    globl_name_to_id: HashMap<String, u32>,
    /// name -> object
    globl_name_to_object: HashMap<String, GloblFuncOrData>,
    /// ID for next private global function
    next_private_globl_funcid: u32,

    /// name -> code
    macro_name_to_lines: HashMap<String, Lines>,
    is_computed_macros: HashMap<String, bool>,

    /// selector code
    entry_point: Lines,
    /// Selector variant
    save_my_code: bool,
    /// Contract version
    version: Option<String>,

    /// starting key for objects in global memory dictionary
    globl_base: Ptr,
    /// key for next object in global memory dictionary
    globl_ptr: Ptr,
    persistent_base: Ptr,
    persistent_ptr: Ptr,

    /// Contract ABI info, used for correct function id calculation
    abi: Option<Contract>,
    // for lazy calculation .compute expressions
    computed: HashMap<String, Lines>,
}

lazy_static! {
    static ref GLOBL_REGEX: Regex = Regex::new(r"^\s*\.globl\s+(:?[\w.]+)").unwrap();
    static ref INTERNAL_REGEX: Regex = Regex::new(r"^\s*\.internal\s+(:\w+)").unwrap();
    static ref LABEL_REGEX: Regex = Regex::new(r"^:?[.\w]+:").unwrap();
    static ref ALIAS_REGEX: Regex = Regex::new(r"^\s*\.internal-alias (:\w+),\s+(-?\d+)").unwrap();
    static ref TYPE_REGEX: Regex = Regex::new(r"^\s*\.type\s+(:?[\w.]+),\s*@([a-zA-Z]+)").unwrap();
    static ref SIZE_REGEX: Regex = Regex::new(r"^\s*\.size\s+([\w.]+),\s*([.\w]+)").unwrap();
    static ref BASE_GLBL_REGEX: Regex = Regex::new(r"^\s*\.global-base\s+(\d+)").unwrap();
    static ref BASE_PERS_REGEX: Regex = Regex::new(r"^\s*\.persistent-base\s+(\d+)").unwrap();
    static ref PUBLIC_REGEX: Regex = Regex::new(r"^\s*\.public\s+([\w.]+)").unwrap();
    static ref MACRO_REGEX: Regex = Regex::new(r"^\s*\.macro\s+([\w.:]+)").unwrap();
    static ref LOC_REGEX: Regex = Regex::new(r"^\s*\.loc\s+(.+),\s+(\d+)\n$").unwrap();
    static ref VERSION_REGEX: Regex = Regex::new(r"^\s*\.version\s+(.+)").unwrap();
    static ref PRAGMA_REGEX: Regex = Regex::new(r"^\s*\.pragma\s+(.+)").unwrap();

    static ref COMPUTE_REGEX: Regex = Regex::new(r"^\s*\.compute\s+\$([\w\.:]+)\$").unwrap();
    static ref CALL_REGEX: Regex = Regex::new(r"^\s*CALL\s+\$([\w\.:]+)\$").unwrap();
}

const GLOBL:            &str = ".globl";
const INTERNAL:         &str = ".internal";
const MACROS:           &str = ".macro";
const SELECTOR:         &str = ".selector";

const DATA_TYPENAME:    &str = "object";

const PERSISTENT_DATA_SUFFIX: &str = "_persistent";

const PUBKEY_NAME:      &str = "tvm_public_key";
const SCI_NAME:         &str = "tvm_contract_info";

fn start_with(sample: &str, pattern: &str) -> bool {
    let s = sample.as_bytes();
    let mut i = 0;
    while i < s.len() && (s[i] == b'\t' || s[i] == b' ') {
        i += 1;
    }
    let p = pattern.as_bytes();
    s.get(i..).unwrap().starts_with(p)
}

pub struct ParseEngineInput<'a> {
    pub buf: Box<dyn Read + 'a>,
    pub name: String,
}

impl ParseEngine {

    pub fn new(sources: Vec<&Path>, abi_json: Option<String>) -> Result<Self> {
        let mut inputs = vec!();
        for path in sources {
            let name = path.file_name().unwrap().to_str().unwrap().to_string();
            let file = File::open(path)
                .map_err(|e| format_err!("Failed to open file {}: {}", path.to_str().unwrap(), e))?;
            inputs.push(ParseEngineInput { buf: Box::new(file), name });
        }
        Self::new_generic(inputs, abi_json)
    }

    pub fn new_generic(inputs: Vec<ParseEngineInput>, abi_json: Option<String>) -> Result<Self> {
        let mut engine = ParseEngine {
            globl_name_to_id:      HashMap::new(),
            internal_name_to_id:    HashMap::new(),
            internal_alias_name_to_id_:    HashMap::new(),
            globl_name_to_object:    HashMap::new(),
            next_private_globl_funcid: 0,
            internal_id_to_code:  HashMap::new(),
            macro_name_to_lines:     HashMap::new(),
            is_computed_macros: HashMap::new(),
            entry_point: vec![],
            globl_base:      0,
            globl_ptr:       0,
            persistent_base: 0,
            persistent_ptr:  0,
            abi:             None,
            version:         None,
            save_my_code:    false,
            computed:        HashMap::new(),
        };
        engine.parse(inputs, abi_json)?;
        Ok(engine)
    }

    fn parse(&mut self, inputs: Vec<ParseEngineInput>, abi_json: Option<String>) -> Status {
        if let Some(s) = abi_json {
            self.abi = Some(load_abi_contract(&s)?);
        }

        self.preinit()?;

        for source in inputs {
            self.parse_code(source)?;
        }

        self.resolve_nested_macros()?;
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
        self.internal_id_to_code.iter().for_each(|x| {
            funcs.insert(*x.0, x.1.body.clone());
        });
        funcs
    }

    fn internal_name(&self, id: i32) -> Option<String> {
        self.internal_name_to_id.iter().find(|i| *i.1 == id).map(|i| i.0.clone())
    }

    fn publics(&self) -> HashMap<u32, Lines> {
        self.globals(true)
    }

    fn privates(&self) -> HashMap<u32, Lines> {
        self.globals(false)
    }

    fn globals(&self, public: bool) -> HashMap<u32, Lines> {
        let mut funcs = HashMap::new();
        let iter = self.globl_name_to_object.iter().filter_map(|item| {
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
        self.globl_name_to_object.iter().find(|item| {
            if let Some(func) = item.1.dtype.func() {
                func.id == id
            } else {
                false
            }
        })
        .map(|i| i.0.clone())
    }

    fn global_by_name(&self, name: &str) -> Option<(u32, Lines)> {
        self.globl_name_to_object.get(name).and_then(|v| {
            v.dtype.func().map(|func| (func.id, func.body.clone()))
        })
    }

    fn preinit(&mut self) -> Status {
        // TODO delete
        self.globl_name_to_object.insert(
            PUBKEY_NAME.to_string(),
            GloblFuncOrData::new(PUBKEY_NAME.to_string(), DATA_TYPENAME)
        );
        self.globl_name_to_object.get_mut(PUBKEY_NAME)
            .unwrap()
            .dtype
            .data_mut()
            .map(|data| {
                data.persistent = true;
                data.values.push(DataValue::Empty);
                data
            });

        // TODO delete
        self.globl_name_to_object.insert(
            SCI_NAME.to_string(),
            GloblFuncOrData::new(SCI_NAME.to_string(), DATA_TYPENAME)
        );
        self.globl_name_to_object.get_mut(SCI_NAME)
            .unwrap()
            .dtype
            .data_mut()
            .map(|data| {
                data.persistent = false;
                data.values.push(DataValue::Empty);
                data
            });
        Ok(())
    }

    fn update_predefined(&mut self) {
        let data = self.globl_name_to_object.get_mut(SCI_NAME)
            .unwrap()
            .dtype
            .data_mut()
            .unwrap();
        data.addr = self.globl_base;

        let data = self.globl_name_to_object.get_mut(PUBKEY_NAME)
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

    fn parse_code(&mut self, mut input: ParseEngineInput) -> Status {
        let mut section_name = String::new();
        let mut obj_body = vec![];
        let mut obj_name = String::new();
        let mut lnum = 0;
        let mut l = String::new();
        let mut source_pos: Option<DbgPos> = None;

        self.globl_ptr = self.globl_base + OFFSET_GLOBL_DATA;
        self.persistent_ptr = self.persistent_base + OFFSET_PERS_DATA;

        let filename = input.name;
        let mut reader = BufReader::new(&mut input.buf);

        while reader.read_line(&mut l)
            .map_err(|e| failure::err_msg(format!("Failed to read file {}: {}", filename.clone(), e)))? != 0 {
            lnum += 1;

            l = l.replace('\r', "");
            if !l.ends_with('\n') {
                l += "\n";
            }

            let pos = match source_pos.clone() {
                None => DbgPos { filename: filename.clone(), line: lnum, line_code: lnum },
                Some(pos) => pos
            };

            if start_with(&l, ".p2align") ||
               start_with(&l, ".align") ||
               start_with(&l, ".text") ||
               start_with(&l, ".file") ||
               start_with(&l, ".ident") ||
               start_with(&l, ".section") {
                //ignore unused parameters
                debug!("ignored: {}", l);
            } else if start_with(&l, ".version") {
                let cap = VERSION_REGEX.captures(&l).unwrap();
                self.version = Some(cap.get(1).unwrap().as_str().to_owned());
            } else if start_with(&l, ".pragma") {
                let cap = PRAGMA_REGEX.captures(&l).unwrap();
                if let Some(m) = cap.get(1) {
                    if m.as_str() == "selector-save-my-code" {
                        self.save_my_code = true
                    }
                }
            } else if start_with(&l, ".global-base") {
                // .global-base
                let cap = BASE_GLBL_REGEX.captures(&l).unwrap();
                let base = cap.get(1).map(|m| m.as_str())
                    .ok_or_else(|| format_err!("line {}: invalid syntax for global base", lnum))?;
                self.globl_base = Ptr::from_str_radix(base, 10)
                    .map_err(|_| format_err!("line {}: invalid global base address", lnum))?;
                self.globl_ptr = self.globl_base + OFFSET_GLOBL_DATA;
                self.update_predefined();
            } else if start_with(&l, ".persistent-base") {
                // .persistent-base
                let cap = BASE_PERS_REGEX.captures(&l).unwrap();
                let base = cap.get(1).map(|m| m.as_str())
                    .ok_or_else(|| format_err!("line {}: invalid syntax for persistent base", lnum))?;
                self.persistent_base = Ptr::from_str_radix(base, 10)
                    .map_err(|_| format_err!("line {}: invalid persistent base address", lnum))?;
                self.persistent_ptr = self.persistent_base + OFFSET_PERS_DATA;
                self.update_predefined();
            } else if start_with(&l, ".type") {
                // .type x, @...
                //it's a mark for beginning of a new object (func or data)
                self.update(&section_name, &obj_name, &obj_body)
                    .map_err(|e| format_err!("line {}: {}", lnum, e))?;
                section_name = GLOBL.to_owned();
                obj_body = vec![];
                let cap = TYPE_REGEX.captures(&l).unwrap();
                obj_name = cap.get(1).unwrap().as_str().to_owned();
                let type_name = cap.get(2).ok_or_else(|| format_err!("line {}: .type option is invalid", lnum))?.as_str();
                let obj = self.globl_name_to_object.entry(obj_name.clone()).or_insert_with(|| GloblFuncOrData::new(obj_name.clone(), type_name));
                obj.dtype = GloblFuncOrDataType::from(type_name);
            } else if start_with(&l, ".size") {
                // .size x, val
                let cap = SIZE_REGEX.captures(&l).unwrap();
                let name = cap.get(1).unwrap().as_str().to_owned();
                let size_str = cap.get(2).ok_or_else(|| format_err!("line {}: .size option is invalid", lnum))?.as_str();
                let item_ref = self.globl_name_to_object.entry(name.clone()).or_insert_with(|| GloblFuncOrData::new(name, ""));
                item_ref.size = size_str.parse::<usize>().unwrap_or(0);
            } else if start_with(&l, ".public") {
                // .public x
                let cap = PUBLIC_REGEX.captures(&l).unwrap();
                let name = cap.get(1).unwrap().as_str();
                self.globl_name_to_object.get_mut(name).map(|obj| { obj.public = true; Some(obj) });
            } else if start_with(&l, ".globl") {
                // .globl x
                let cap = GLOBL_REGEX.captures(&l).unwrap();
                let name = cap.get(1).unwrap().as_str().to_owned();
                self.globl_name_to_object.entry(name.clone()).or_insert_with(|| GloblFuncOrData::new(name.clone(), ""));
            } else if start_with(&l, ".macro") {
                // .macro x
                self.update(&section_name, &obj_name, &obj_body)
                    .map_err(|e| format_err!("line {}: {}", lnum, e))?;
                section_name = MACROS.to_owned();
                obj_body = vec![];
                obj_name = MACRO_REGEX.captures(&l).unwrap().get(1).unwrap().as_str().to_owned();
            } else if start_with(&l, ".data") {
                // .data
                //ignore, not used
            } else if start_with(&l, ".selector") {
                // .selector
                self.update(&section_name, &obj_name, &obj_body)?;
                section_name = SELECTOR.to_owned();
                obj_name = "".to_owned();
                obj_body = vec![];
            } else if start_with(&l, ".internal-alias") {
                // .internal-alias
                let cap = ALIAS_REGEX.captures(&l).unwrap();
                self.internal_alias_name_to_id_.insert(
                    cap.get(1).unwrap().as_str().to_owned(),
                    cap.get(2).unwrap().as_str().parse::<i32>()
                        .map_err(|_| format_err!("line: '{}': failed to parse id", lnum))?,
                );
            } else if start_with(&l, ".internal") {
                // .internal
                self.update(&section_name, &obj_name, &obj_body)
                    .map_err(|e| format_err!("line {}: {}", lnum, e))?;
                section_name = INTERNAL.to_owned();
                obj_body = vec![];
                obj_name = INTERNAL_REGEX.captures(&l).unwrap().get(1).unwrap().as_str().to_owned();
            } else if LABEL_REGEX.is_match(&l) {
                // TODO
                // ignore labels
            } else if start_with(&l, ".loc") {
                let cap = LOC_REGEX.captures(&l).unwrap();
                let filename = String::from(cap.get(1).unwrap().as_str());
                let line = cap.get(2).unwrap().as_str().parse::<usize>().unwrap();
                if line == 0 { // special value for resetting current source pos
                    source_pos = None;
                } else {
                    source_pos = Some(DbgPos { filename, line, line_code: lnum });
                }
            } else if
                start_with(&l, ".blob") ||
                start_with(&l, ".cell") ||
                start_with(&l, ".byte") ||
                start_with(&l, ".long") ||
                start_with(&l, ".short") ||
                start_with(&l, ".quad") ||
                start_with(&l, ".comm") ||
                start_with(&l, ".bss") ||
                start_with(&l, ".asciz") ||
                start_with(&l, ".compute") {
                // .param [value]
                obj_body.push(Line { text: l.clone(), pos })
            } else {
                obj_body.push(Line { text: l.clone(), pos });
            }
            l.clear();
        }

        if section_name.is_empty() {
            bail!("input file has no assembler definitions");
        }

        self.update(&section_name, &obj_name, &obj_body)
            .map_err(|e| format_err!("line {}: {}", lnum, e))?;
        Ok(())
    }

    fn resolve_nested_macros_in_lines(&mut self, lines: Lines) -> Result<Lines> {
        let mut new_lines: Lines = vec![];
        for line in lines {
            if start_with(&line.text, "CALL $") {
                let next_name = CALL_REGEX.captures(&line.text).unwrap().get(1).unwrap().as_str().to_string();
                if self.macro_name_to_lines.get(&next_name).is_some() {
                    self.resolve_nested_macro(&next_name)?;
                    let mut resolved_lines = self.macro_name_to_lines.get(&next_name).unwrap().clone();
                    new_lines.append(&mut resolved_lines);
                    continue
                }
            }
            new_lines.push(line);
        }
        Ok(new_lines)
    }

    fn resolve_nested_macros(&mut self) -> Status {
        let names = self.globl_name_to_object.keys().cloned().collect::<Vec<_>>();
        for name in &names {
            if let GloblFuncOrDataType::Function(f) = &self.globl_name_to_object.get_mut(name).unwrap().dtype {
                let lines = f.body.clone();
                let new_lines = self.resolve_nested_macros_in_lines(lines)?;
                self.globl_name_to_object.get_mut(name).unwrap().dtype.func_mut().unwrap().body = new_lines;
            }
        }

        let ids = self.internal_id_to_code.keys().copied().collect::<Vec<_>>();
        for id in &ids {
            let lines = self.internal_id_to_code.get(id).unwrap().body.clone();
            let new_lines = self.resolve_nested_macros_in_lines(lines)?;
            self.internal_id_to_code.get_mut(id).unwrap().body = new_lines;
        }

        Ok(())
    }

    fn resolve_nested_macro(&mut self, name: &str) -> Status {
        if let Some(is_computed) = self.is_computed_macros.get(name) {
            return if *is_computed {
                Ok(())
            } else {
                Err(format_err!("Internal error. Macros have a cycle. See {}", name))
            }
        }
        self.is_computed_macros.insert(name.to_string(), false);
        let lines = self.macro_name_to_lines.get(name).unwrap().clone();
        let new_lines = self.resolve_nested_macros_in_lines(lines)?;
        self.macro_name_to_lines.insert(name.to_string(), new_lines);
        self.is_computed_macros.insert(name.to_string(), true);
        Ok(())
    }

    fn replace_labels_in_body(&mut self, lines: Lines, obj_name: FunctionId) -> Result<Lines> {
        let mut new_lines = vec![];
        for line in lines {
            if start_with(&line.text, ".compute") {
                let name = COMPUTE_REGEX.captures(&line.text).unwrap().get(1).unwrap().as_str();
                let mut resolved = self.compute_cell(name)?;
                new_lines.append(&mut resolved);
                continue
            }

            let resolved =
                self.replace_labels(&line, &obj_name)
                    .map_err(|e| format_err!("line {}: cannot resolve label: {}", line.pos.line, e))?;
            new_lines.push(resolved);
        }
        Ok(new_lines)
    }

    fn replace_all_labels(&mut self) -> Status {
        let names = self.globl_name_to_object.keys().cloned().collect::<Vec<_>>();
        for name in &names {
            if let GloblFuncOrDataType::Function(f) = &self.globl_name_to_object.get(name).unwrap().dtype {
                let lines = f.body.clone();
                let obj_name = self.globl_name_to_object.get(name).unwrap().name.clone();
                let new_lines = self.replace_labels_in_body(lines, FunctionId::Name(obj_name))?;

                let body = &mut self.globl_name_to_object.get_mut(name).unwrap().dtype.func_mut().unwrap().body;
                *body = new_lines;
            }
        }

        let ids = self.internal_id_to_code.keys().copied().collect::<Vec<_>>();
        for id in &ids {
            let lines = self.internal_id_to_code.get(id).unwrap().body.clone();
            let new_lines = self.replace_labels_in_body(lines, FunctionId::Id(*id))?;

            let body = &mut self.internal_id_to_code.get_mut(id).unwrap().body;
            *body = new_lines;
        }

        Ok(())
    }

    fn create_function_id(&mut self, func: &str) -> u32 {
        let is_public = self.globl_name_to_object.get(func).unwrap().public;
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

    fn update(&mut self, section: &str, name: &str, body: &Lines) -> Status {
        match section {
            SELECTOR => {
                if self.entry_point.is_empty() {
                    self.entry_point = body.clone();
                } else {
                    bail!("Another selector found");
                }
            },
            GLOBL => {
                //do not reset public flag if symbol isn't included in ABI,
                //because it can be marked as public in assembly.
                if self.is_public(name) {
                    self.globl_name_to_object.get_mut(name).unwrap().public = true;
                }

                if self.globl_name_to_object.get(name).unwrap().dtype.is_func() {
                    // globl object is function
                    let func_id = self.create_function_id(name);
                    let item = self.globl_name_to_object.get_mut(name).unwrap();
                    let params = item.dtype.func_mut().unwrap();
                    params.id = func_id;
                    params.body = body.clone();
                    let prev = self.globl_name_to_id.insert(name.to_string(), func_id);
                    if prev.is_some() {
                        bail!(
                            "global function with id = {:x} and name \"{}\" already exist",
                            func_id,
                            name,
                        );
                    }
                } else {
                    // globl object is data
                    let item = self.globl_name_to_object.get_mut(name).unwrap();
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
                let func_id = self.internal_alias_name_to_id_.get(name).ok_or_else(|| format_err!("id for '{}' not found", name))?;
                self.internal_name_to_id.insert(name.to_string(), *func_id);
                let prev = self.internal_id_to_code.insert(*func_id, InternalFunc::new());
                if prev.is_some() {
                    bail!("internal function with id = {} already exist", *func_id);
                }
                self.internal_id_to_code.get_mut(func_id).unwrap().body = body.clone();
            },
            MACROS => {
                let prev = self.macro_name_to_lines.insert(name.to_string(), body.clone());
                if prev.is_some() {
                    bail!("macros with name \"{}\" already exist", name);
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
    ) -> Status {
        lazy_static! {
            static ref PARAM_RE: Regex = Regex::new(r#"^\s+\.(\w+),?\s*([a-zA-Z0-9-_\s"]+)"#).unwrap();
            static ref COMM_RE:  Regex = Regex::new(r"^\s*\.comm\s+([\w\.]+),\s*(\d+),\s*(\d+)").unwrap();
            static ref ASCI_RE:  Regex = Regex::new(r#"^\s*\.asciz\s+"(.+)""#).unwrap();
        }
        for param in body {
            let mut value_len: usize = 0;
            if let Some(cap) = COMM_RE.captures(param.text.as_str()) {
                // .comm <symbol>, <size>, <align>
                let size_bytes = cap.get(2).unwrap().as_str().parse::<usize>()
                    .map_err(|_| format_err!("invalid \".comm\": invalid size"))?;
                let align = cap.get(3).unwrap().as_str().parse::<usize>()
                    .map_err(|_| format_err!("\".comm\": invalid align"))?;

                if size_bytes == 0  {
                    bail!("\".comm\": invalid size".to_string());
                }
                if (align == 0) || (align % WORD_SIZE as usize != 0) {
                    bail!("\".comm\": invalid align".to_string());
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
                    _ => bail!("invalid parameter: \"{}\"", param.text),
                };
                let value = cap.get(2).map_or("", |m| m.as_str()).trim();
                values.push(DataValue::Number((
                    IntegerData::from_str_radix(value, 10)
                        .map_err(|_| format_err!("parameter \"{}\" has invalid value \"{}\"", pname, value))?,
                    value_len,
                )));
            }
            if *item_size < value_len {
                bail!("global object {} has invalid .size parameter: too small", name);
            }
            *item_size -= value_len;
        }
        if *item_size > 0 {
            bail!("global object {} has invalid \".size\" value: real size = {}", name, *item_size);
        }
        Ok(())
    }

    pub fn build_data(&self) -> Option<Cell> {
        let filter = |persistent: bool| {
            self.globl_name_to_object.iter().filter_map(move |item| {
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
                let mut ptr = *item.0;
                for subitem in item.1 {
                    dict.set(ptr_to_builder(ptr).unwrap().into_cell().unwrap().into(), &subitem.write().unwrap_or_default().into_cell().unwrap().into()).unwrap();
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

        pers_dict.data().cloned()
    }

    fn encode_computed_cell(&self, cell: &Cell, toplevel: bool) -> Lines {
        let slice = SliceData::from(cell);
        let mut lines = vec!();
        let opening = if toplevel { "{\n" } else { ".cell {\n" };
        lines.push(Line::new(opening, "", 0));
        lines.push(Line::new(format!(".blob x{}\n", slice.to_hex_string()).as_str(), "", 0));
        for i in slice.get_references() {
            let child = cell.reference(i).unwrap();
            let mut child_lines = self.encode_computed_cell(&child, false);
            lines.append(&mut child_lines);
        }
        lines.push(Line::new("}\n", "", 0));
        lines
    }

    fn compute_cell(&mut self, name: &str) -> Result<Lines> {
        if let Some(computed) = self.computed.get(name) {
            return Ok(computed.clone())
        }

        let lines = self.macro_name_to_lines.get(name).ok_or_else(|| format_err!("macro {} was not found", name))?.clone();

        let mut collected = vec!();
        for line in lines {
            if COMPUTE_REGEX.is_match(&line.text) {
                let name_inner = COMPUTE_REGEX.captures(&line.text).unwrap().get(1).unwrap().as_str();
                collected.append(&mut self.compute_cell(name_inner)?);
            } else {
                collected.push(line.clone());
            }
        }

        let (code, _) = ton_labs_assembler::compile_code_debuggable(collected)
            .map_err(|e| format_err!("{}", e))?;

        let mut engine = ton_vm::executor::Engine::with_capabilities(0).setup_with_libraries(
            code, None, None, None, vec![]);
        match engine.execute() {
            Err(e) => {
                println!("failed to compute cell: {}", e);
                bail!(name.to_string())
            }
            Ok(code) => {
                if code != 0 {
                    println!("failed to compute cell, exit code {}", code);
                    bail!(name.to_string())
                }
            }
        };

        let cell = engine.stack().get(0).as_cell().map_err(|e| format_err!("{}: {}", name, e))?;
        let res = self.encode_computed_cell(cell, true);
        self.computed.insert(String::from(name), res.clone());
        Ok(res)
    }

    fn replace_labels(&mut self, line: &Line, cur_obj_name: &FunctionId) -> Result<Line> {
        resolve_name(line, |name| {
            self.internal_name_to_id.get(name).copied()
        })
        .or_else(|_| resolve_name(line, |name| {
            self.globl_name_to_id.get(name).copied().map(|id| {
                self.insert_called_func(cur_obj_name, id);
                id
            })
        }))
        .or_else(|_| resolve_name(line, |name| {
            self.globl_name_to_object.get(name).and_then(|obj| {
                obj.dtype.data().map(|data| data.addr)
            })
        }))
        .or_else(|_| resolve_name(line, |name| {
            match name {
                "global-base" => Some(self.globl_base),
                "persistent-base" => Some(self.persistent_base),
                _ => None,
            }
        }))
    }

    fn insert_called_func(&mut self, from_func: &FunctionId, to_func: u32) {
        match from_func {
            FunctionId::Name(name) => {
                self.globl_name_to_object.get_mut(name)
                    .and_then(|obj| {
                        obj.dtype.func_mut().map(|f| {
                            f.called_ids.push(to_func);
                            Some(f)
                        })
                    });
            }
            FunctionId::Id(id) => {
                self.internal_id_to_code.get_mut(id).map(|f| {
                    f.called_ids.push(to_func);
                    Some(f)
                });
            }
        }
    }

    fn drop_unused_objects(&mut self) {
        let mut ids = HashSet::new();
        let publics_iter = self.globl_name_to_object.iter().filter_map(|obj| {
            obj.1.dtype.func()
                .and_then(|i| if obj.1.public { Some(i) } else { None })
        });

        for func in publics_iter {
            self.enum_calling_funcs(func, &mut ids);
        }
        for func in self.internal_id_to_code.iter() {
            self.enum_calling_funcs(func.1, &mut ids);
        }

        self.globl_name_to_object.retain(|_k, v| {
            v.dtype.func()
                .map(|f| ids.contains(&f.id))
                .unwrap_or(true)
        });
        self.globl_name_to_id.retain(|_k, v| {
            ids.contains(v)
        });
    }

    fn enum_calling_funcs(&self, func: &InternalFunc, ids: &mut HashSet<u32>) {
        ids.insert(func.id); // TODO there are public/private globs and internals

        for id in &func.called_ids {
            if ids.insert(*id) {
                let subfunc = self.globl_name_to_object.iter().find(|(_name, obj)| {
                    obj.dtype.func().map(|f| f.id == *id).unwrap_or(false)
                })
                    .map(|(_name, obj)| obj.dtype.func().unwrap());
                if let Some(subfunc) = subfunc {
                    self.enum_calling_funcs(subfunc, ids);
                }
            }
        }
    }

    fn debug_print(&self) {
        let line = "--------------------------";
        let entry = lines_to_string(&self.entry());
        println!("Entry point:\n{}\n{}\n{}", line, entry, line);
        println!("General-purpose functions:\n{}", line);

        let mut keys = self.globl_name_to_id.keys().collect::<Vec<_>>();
        keys.sort();
        for k in keys {
            println! ("Function {:30}: id={:08X} public={}",
                      k,
                      self.globl_name_to_id.get(k).unwrap(),
                      self.globl_name_to_object.get(k).unwrap().public);
        }
        println!("private:");
        for (k, v) in &self.privates() {
            let code = lines_to_string(v);
            println! ("Function {:08X}\n{}\n{}\n{}", k, line, code, line);
        }
        println!("public:");
        for (k, v) in self.publics() {
            let code = lines_to_string(&v);
            println! ("Function {:08X}\n{}\n{}\n{}", k, line, code, line);
        }
        println!("{}\nInternal functions:\n{}", line, line);
        for (k, v) in &self.internal_name_to_id {
            println! ("Function {:30}: id={:08X}", k, v);
        }
        for (k, v) in &self.internal_id_to_code {
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

        let mut engine = Engine::with_capabilities(0).setup_with_libraries(
            code, None, Some(stack), None, vec![]
        );
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

    #[test]
    fn test_compute_nested() {
        let sources = vec![Path::new("./tests/test_stdlib.tvm"),
                                     Path::new("./tests/test_compute_nested.code")];
        let parser = ParseEngine::new(sources, None);
        assert_eq!(parser.is_ok(), true);

        let internals = parser.unwrap().internals();
        let internal = internals.get(&-2).unwrap();

        assert_eq!(
            *internal,
            vec![
                Line::new("\n",               "test_compute_nested.code", 3),
                Line::new("PUSHREF\n",        "test_compute_nested.code", 4),
                Line::new("{\n",                "", 0),
                Line::new(".blob x0000006f00000000000000de\n", "", 0),
                Line::new("}\n",                "", 0),
                Line::new("DROP\n",           "test_compute_nested.code", 6),
                Line::new("\n",               "test_compute_nested.code", 7),
            ]
        );
    }
}
