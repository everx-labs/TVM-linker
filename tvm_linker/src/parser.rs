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
use std::collections::{HashSet, HashMap};
use std::io::{BufRead, BufReader};
use std::fs::File;
use std::path::Path;
use ton_types::{BuilderData, IBitstring, SliceData, Cell};
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

enum FunctionId {
    Name(String),
    Id(i32)
}

struct GloblFunc {
    pub name: String,
    pub public: bool,
    pub dtype: InternalFunc,
}

impl GloblFunc {
    pub fn new(name: String) -> Self {
        GloblFunc {
            name,
            public: false,
            dtype: InternalFunc::new(),
        }
    }
}

impl Default for GloblFunc {
    fn default() -> Self {
        GloblFunc::new(String::new())
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
    /// aliases for function names, e.g. main_internal -> 0, main_internalXXX -> 0,
    internal_alias_name_to_id_: HashMap<String, i32>, // TODO delete this or internal_name_to_id

    /// it's about private/public .globl or variables, e.g.
    /// .globl	sendMessage_internal
    /// .type	sendMessage_internal, @function
    /// ...
    /// name -> id
    globl_name_to_id: HashMap<String, u32>,
    /// name -> object
    globl_name_to_func: HashMap<String, GloblFunc>,
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

    /// Contract ABI info, used for correct function id calculation
    abi: Option<Contract>,
    // for lazy calculation .compute expressions
    computed: HashMap<String, Lines>,
}

const PATTERN_GLOBL:    &'static str = r"^\s*\.globl\s+(:?[\w\.]+)";
const PATTERN_INTERNAL: &'static str = r"^\s*\.internal\s+(:\w+)";
const PATTERN_SELECTOR: &'static str = r"^\s*\.selector";
const PATTERN_ALIAS:    &'static str = r"^\s*\.internal-alias (:\w+),\s+(-?\d+)";
const PATTERN_LABEL:    &'static str = r"^:?[\.\w]+:";
const PATTERN_TYPE:     &'static str = r"^\s*\.type\s+(:?[\w\.]+),\s*@([a-zA-Z]+)";
const PATTERN_PUBLIC:   &'static str = r"^\s*\.public\s+([\w\.]+)";
const PATTERN_MACRO:    &'static str = r"^\s*\.macro\s+([\w\.:]+)";
const PATTERN_IGNORED:  &'static str = r"^\s+\.(size|persistent-base|global-base|data|p2align|align|text|file|ident|section)";
const PATTERN_LOC:      &'static str = r"^\s*\.loc\s+(.+),\s+(\d+)\n$";
const PATTERN_VERSION:  &'static str = r"^\s*\.version\s+(.+)";
const PATTERN_PRAGMA:   &'static str = r"^\s*\.pragma\s+(.+)";

lazy_static! {
    // TODO move all here
    static ref COMPUTE_REGEX: Regex = Regex::new(r"^\s*\.compute\s+\$([\w\.:]+)\$").unwrap();
    static ref CALL_REGEX: Regex = Regex::new(r"^\s*CALL\s+\$([\w\.:]+)\$").unwrap();
    static ref NAMES: Regex = Regex::new(r"(?P<head>.*)\$(?P<mid>.*)\$(?P<tail>.*)").unwrap();
}

const GLOBL:            &'static str = ".globl";
const INTERNAL:         &'static str = ".internal";
const MACROS:           &'static str = ".macro";
const SELECTOR:         &'static str = ".selector";


impl ParseEngine {

    pub fn new(sources: Vec<&Path>, abi_json: Option<String>) -> Result<Self, String> {
        let mut engine = ParseEngine {
            globl_name_to_id: HashMap::new(),
            internal_name_to_id: HashMap::new(),
            internal_alias_name_to_id_: HashMap::new(),
            globl_name_to_func: HashMap::new(),
            next_private_globl_funcid: 0,
            internal_id_to_code: HashMap::new(),
            macro_name_to_lines: HashMap::new(),
            is_computed_macros: HashMap::new(),
            entry_point: vec![],
            abi: None,
            version: None,
            save_my_code: false,
            computed: HashMap::new(),
        };
        engine.parse(sources, abi_json)?;
        Ok(engine)
    }

    fn parse(&mut self, sources: Vec<&Path>, abi_json: Option<String>) -> Result<(), String> {
        if let Some(s) = abi_json {
            self.abi = Some(load_abi_contract(&s)?);
        }

        for source in &sources {
            self.parse_code(source)?;
        }

        self.resolve_nested_macros()?;
        self.replace_all_labels()?;

        self.drop_unused_objects();
        Ok(())
    }

    fn entry(&self) -> Lines {
        self.entry_point.clone()
    }

    fn internals(&self) -> HashMap<i32, Lines> {
        let mut funcs = HashMap::new();
        self.internal_id_to_code.iter().for_each(|x| {
            funcs.insert(x.0.clone(), x.1.body.clone());
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
        for i in &self.globl_name_to_func {
            if i.1.public == public {
                funcs.insert(i.1.dtype.id, i.1.dtype.body.clone());
            }
        }
        funcs
    }

    fn global_name(&self, id: u32) -> Option<String> {
        self.globl_name_to_func.iter().find(|item| {
            item.1.dtype.id == id
        })
        .map(|i| i.0.clone())
    }

    fn global_by_name(&self, name: &str) -> Option<(u32, Lines)> {
        self.globl_name_to_func.get(name).and_then(|v| {
            Some((v.dtype.id.clone(), v.dtype.body.clone()))
        })
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
        let label_regex = Regex::new(PATTERN_LABEL).unwrap();
        let alias_regex = Regex::new(PATTERN_ALIAS).unwrap();
        let type_regex = Regex::new(PATTERN_TYPE).unwrap();
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
            } else if let Some(cap) = version_regex.captures(&l) {
                self.version = Some(cap.get(1).unwrap().as_str().to_owned());
            } else if let Some(cap) = pragma_regex.captures(&l) {
                match cap.get(1) {
                    Some(m) => if m.as_str() == "selector-save-my-code" {
                        self.save_my_code = true
                    },
                    None => {}
                }
            } else if let Some(cap) = type_regex.captures(&l) {
                // .type x, @...
                //it's a mark of beginning of a new function
                self.update(&section_name, &obj_name, &obj_body)
                    .map_err(|e| format!("line {}: {}", lnum, e))?;
                section_name = GLOBL.to_owned();
                obj_body = vec![];
                obj_name = cap.get(1).unwrap().as_str().to_owned();
                let obj = self.globl_name_to_func.entry(obj_name.clone()).or_insert(GloblFunc::new(obj_name.clone()));
                obj.dtype = InternalFunc::new();
            } else if let Some(cap) = public_regex.captures(&l) {
                // .public x
                let name = cap.get(1).unwrap().as_str();
                self.globl_name_to_func.get_mut(name).and_then(|obj| {obj.public = true; Some(obj)});
            } else if let Some(cap) = globl_regex.captures(&l) {
                // .globl x
                let name = cap.get(1).unwrap().as_str().to_owned();
                self.globl_name_to_func.entry(name.clone()).or_insert(GloblFunc::new(name.clone()));
            } else if let Some(cap) = macro_regex.captures(&l) {
                // .macro x
                self.update(&section_name, &obj_name, &obj_body)
                    .map_err(|e| format!("line {}: {}", lnum, e))?;
                section_name = MACROS.to_owned();
                obj_body = vec![];
                obj_name = cap.get(1).unwrap().as_str().to_owned();
            } else if selector_regex.is_match(&l) {
                // .selector
                self.update(&section_name, &obj_name, &obj_body)?;
                section_name = SELECTOR.to_owned();
                obj_name = "".to_owned();
                obj_body = vec![];
            } else if let Some(cap) = internal_regex.captures(&l) {
                // .internal
                self.update(&section_name, &obj_name, &obj_body)
                    .map_err(|e| format!("line {}: {}", lnum, e))?;
                section_name = INTERNAL.to_owned();
                obj_body = vec![];
                obj_name = cap.get(1).unwrap().as_str().to_owned();
            } else if let Some(cap) = alias_regex.captures(&l) {
                // .internal-alias
                self.internal_alias_name_to_id_.insert(
                    cap.get(1).unwrap().as_str().to_owned(),
                    i32::from_str_radix(cap.get(2).unwrap().as_str(), 10)
                        .map_err(|_| format!("line: '{}': failed to parse id", lnum))?,
                );
            } else if label_regex.is_match(&l) {
                // ignore labels
            } else if let Some(cap) = loc_regex.captures(&l) {
                let filename = String::from(cap.get(1).unwrap().as_str());
                let line = cap.get(2).unwrap().as_str().parse::<usize>().unwrap();
                if line == 0 { // special value for resetting current source pos
                    source_pos = None;
                } else {
                    source_pos = Some(DbgPos { filename, line, line_code: lnum });
                }
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

    fn resolve_nested_macros_in_lines(&mut self, lines: Lines) -> Result<Lines, String> {
        let mut new_lines: Lines = vec![];
        for line in lines {
            if let Some(captures) = CALL_REGEX.captures(&line.text) {
                let next_name = captures.get(1).unwrap().as_str().to_string();
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

    fn resolve_nested_macros(&mut self) -> Result<(), String> {
        let names = self.globl_name_to_func.keys().map(|k| k.clone()).collect::<Vec<_>>();
        for name in &names {
            let f = &self.globl_name_to_func.get_mut(name).unwrap().dtype;
            let lines = f.body.clone();
            let new_lines = self.resolve_nested_macros_in_lines(lines)?;
            self.globl_name_to_func.get_mut(name).unwrap().dtype.body = new_lines;
        }

        let ids = self.internal_id_to_code.keys().map(|x| *x).collect::<Vec<_>>();
        for id in &ids {
            let lines = self.internal_id_to_code.get(id).unwrap().body.clone();
            let new_lines = self.resolve_nested_macros_in_lines(lines)?;
            self.internal_id_to_code.get_mut(id).unwrap().body = new_lines;
        }

        Ok(())
    }

    fn resolve_nested_macro(&mut self, name: &String) -> Result<(), String> {
        if let Some(is_computed) = self.is_computed_macros.get(name) {
            return if *is_computed {
                Ok(())
            } else {
                Err("Internal error. Macros have a cycle. See ".to_string() + name)
            }
        }
        self.is_computed_macros.insert(name.to_string(), false);
        let lines = self.macro_name_to_lines.get(name).unwrap().clone();
        let new_lines = self.resolve_nested_macros_in_lines(lines)?;
        self.macro_name_to_lines.insert(name.to_string(), new_lines);
        self.is_computed_macros.insert(name.to_string(), true);
        Ok(())
    }

    fn replace_labels_in_body(&mut self, lines: Lines, obj_name: FunctionId) -> Result<Lines, String> {
        let mut new_lines = vec![];
        for line in lines {
            if let Some(captures) = COMPUTE_REGEX.captures(&line.text) {
                let name = captures.get(1).unwrap().as_str();
                let mut resolved = self.compute_cell(name)?;
                new_lines.append(&mut resolved);
                continue
            }

            let mut resolved =
                self.replace_labels(&line, &obj_name)
                    .map_err(|e| format!("line {}: cannot resolve label: {}", line.pos.line, e))?;
            new_lines.append(&mut resolved);
        }
        Ok(new_lines)

    }

    fn replace_all_labels(&mut self) -> Result<(), String> {
        let names = self.globl_name_to_func.keys().map(|k| k.clone()).collect::<Vec<_>>();
        for name in &names {
            let f = &self.globl_name_to_func.get(name).unwrap().dtype;
            let lines = f.body.clone();
            let obj_name = self.globl_name_to_func.get(name).unwrap().name.clone();
            let new_lines = self.replace_labels_in_body(lines, FunctionId::Name(obj_name))?;

            let body = &mut self.globl_name_to_func.get_mut(name).unwrap().dtype.body;
            *body = new_lines;
        }

        let ids = self.internal_id_to_code.keys().map(|x| *x).collect::<Vec<_>>();
        for id in &ids {
            let lines = self.internal_id_to_code.get(id).unwrap().body.clone();
            let new_lines = self.replace_labels_in_body(lines, FunctionId::Id(*id))?;

            let body = &mut self.internal_id_to_code.get_mut(id).unwrap().body;
            *body = new_lines;
        }

        Ok(())
    }

    fn create_function_id(&mut self, func: &str) -> u32 {
        let is_public = self.globl_name_to_func.get(func).unwrap().public;
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
                // TODO really?
                if self.is_public(name) {
                    self.globl_name_to_func.get_mut(name).unwrap().public = true;
                }

                // globl object is function
                let func_id = self.create_function_id(name);
                let item = self.globl_name_to_func.get_mut(name).unwrap();
                let params = &mut item.dtype;
                params.id = func_id;
                params.body = body.clone();
                let prev = self.globl_name_to_id.insert(name.to_string(), func_id);
                if prev.is_some() {
                    Err(format!(
                        "global function with id = {:x} and name \"{}\" already exist",
                        func_id,
                        name,
                    ))?;
                }
            },
            INTERNAL => {
                let func_id = self.internal_alias_name_to_id_.get(name).ok_or(format!("id for '{}' not found", name))?;
                self.internal_name_to_id.insert(name.to_string(), *func_id);
                let prev = self.internal_id_to_code.insert(*func_id, InternalFunc::new());
                if prev.is_some() {
                    Err(format!("internal function with id = {} already exist", *func_id))?;
                }
                self.internal_id_to_code.get_mut(func_id).unwrap().body = body.clone();
            },
            MACROS => {
                let prev = self.macro_name_to_lines.insert(name.to_string(), body.clone());
                if prev.is_some() {
                    Err(format!("macros with name \"{}\" already exist", name))?;
                }
            },
            _ => (),
        }
        Ok(())
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

    fn compute_cell(&mut self, name: &str) -> Result<Lines, String> {
        if let Some(computed) = self.computed.get(name) {
            return Ok(computed.clone())
        }

        let lines = self.macro_name_to_lines.get(name).ok_or(format!("macro {} was not found", name))?.clone();

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
        let res = self.encode_computed_cell(cell, true);
        self.computed.insert(String::from(name), res.clone());
        Ok(res)
    }

    fn replace_labels(&mut self, line: &Line, cur_obj_name: &FunctionId) -> Result<Lines, String> {
        if let Some(cap) = NAMES.captures(&line.text) {
            let name = cap.get(2).unwrap().as_str().to_string();
            let id_str;
            if let Some(id) = self.globl_name_to_id.get(&name) {
                id_str = id.to_string();
                let local_id = *id;
                self.insert_called_func(&cur_obj_name, local_id);
            } else if let Some(id) = self.internal_name_to_id.get(&name) {
                id_str = id.to_string();
            } else {
                return Err("Internal error in resolving ".to_string() + &line.text);
            }
            let mut res = [cap.get(1).unwrap().as_str(), id_str.to_string().as_str(), cap.get(3).unwrap().as_str()].join("");
            if !res.ends_with('\n') { // TODO why here?
                res += "\n";
            }
            return Ok(vec![Line::new(res.as_str(), line.pos.filename.as_str(), line.pos.line)]);
        }
        return Ok(vec![line.clone()]);
    }

    fn insert_called_func(&mut self, from_func: &FunctionId, to_func: u32) {
        match from_func {
            FunctionId::Name(name) => {
                self.globl_name_to_func.get_mut(name)
                    .and_then(|obj| {
                        obj.dtype.called_ids.push(to_func);
                        Some(obj)
                    });
            }
            FunctionId::Id(id) => {
                self.internal_id_to_code.get_mut(&id).and_then(|f| {
                    f.called_ids.push(to_func);
                    Some(f)
                });
            }
        }
    }

    fn drop_unused_objects(&mut self) {
        let mut ids = HashSet::new();
        let publics_iter = self.globl_name_to_func.iter().filter_map(|obj| {
            if obj.1.public { Some(&obj.1.dtype) } else { None }
        });

        for func in publics_iter {
            self.enum_calling_funcs(&func, &mut ids);
        }
        for func in self.internal_id_to_code.iter() {
            self.enum_calling_funcs(&func.1, &mut ids);
        }

        self.globl_name_to_func.retain(|_k, v| {
            ids.contains(&v.dtype.id)
        });
        self.globl_name_to_id.retain(|_k, v| {
            ids.contains(&v)
        });
    }

    fn enum_calling_funcs(&self, func: &InternalFunc, ids: &mut HashSet<u32>) {
        ids.insert(func.id); // TODO there are public/private globs and internals

        for id in &func.called_ids {
            if ids.insert(*id) {
                let subfunc = self.globl_name_to_func.iter().find(|(_name, obj)| {
                    obj.dtype.id == *id
                })
                    .map(|(_name, obj)| &obj.dtype);
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

        let mut keys = self.globl_name_to_id.keys().collect::<Vec<_>>();
        keys.sort();
        for k in keys {
            println! ("Function {:30}: id={:08X} public={}",
                      k,
                      self.globl_name_to_id.get(k).unwrap(),
                      self.globl_name_to_func.get(k).unwrap().public);
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
        let internal = globals.get(&1).unwrap();

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
                 Line::new("CALL 1\n",     "test_macros_02.code", 8),
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
