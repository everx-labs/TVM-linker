/*
 * Copyright 2018-2021 TON DEV SOLUTIONS LTD.
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

use std::collections::HashMap;
use ton_types::{Cell, Result, /*Bitmask,*/ SliceData, fail};

pub type Code = Vec<Instruction>;

#[derive(Debug)]
pub struct Instruction {
    name: &'static str,
    params: Vec<InstructionParameter>,
    quiet: bool,
}

impl Instruction {
    pub fn new(name: &'static str) -> Self {
        Self { name, params: vec!(), quiet: false }
    }
    pub fn with_param(self, param: InstructionParameter) -> Self {
        let mut clone = self;
        clone.params.push(param);
        clone
    }
    pub fn set_quiet(self) -> Self {
        let mut clone = self;
        clone.quiet = true;
        clone
    }
    pub fn name(&self) -> &'static str {
        self.name
    }
    pub fn params(&self) -> &Vec<InstructionParameter> {
        &self.params
    }
    pub fn is_quiet(&self) -> bool {
        self.quiet
    }
}

#[derive(Debug)]
pub enum InstructionParameter {
    BigInteger(num::BigInt),
    ControlRegister(usize),
    //DivisionMode(DivMode),
    Integer(isize),
    Length(usize),
    LengthAndIndex(usize, usize),
    Nargs(isize),
    Pargs(usize),
    Rargs(usize),
    Slice(SliceData),
    StackRegister(isize),
    StackRegisterPair(isize, isize),
    StackRegisterTriple(isize, isize, isize),
    Code(Code),
}

// #[derive(Clone, Debug)]
// pub struct DivMode {
//     flags: Bitmask,
// }

pub(super) trait OperationBehavior {
    fn insn(insn: Instruction) -> Instruction;
}
pub(super) struct Signaling {}
pub(super) struct Quiet {}
impl OperationBehavior for Signaling {
    fn insn(insn: Instruction) -> Instruction {
        insn
    }
}
impl OperationBehavior for Quiet {
    fn insn(insn: Instruction) -> Instruction {
        insn.set_quiet()
    }
}

enum ShapeKind {
    Any,
    Literal(Vec<u8>),
    Var(&'static str),
}

pub struct Shape {
    kind: ShapeKind,
    refs: Vec<Shape>,
}

impl Shape {
    pub fn any() -> Shape {
        Shape { kind: ShapeKind::Any, refs: vec![] }
    }
    pub fn literal(cst: &'static str) -> Shape {
        Shape { kind: ShapeKind::Literal(hex::decode(cst).expect("bad literal")), refs: vec![] }
    }
    pub fn var(name: &'static str) -> Shape {
        Shape { kind: ShapeKind::Var(name), refs: vec![] }
    }
    pub fn branch(self, node: Shape) -> Shape {
        let mut copy = self;
        copy.refs.push(node);
        copy
    }
    pub fn captures(&self, cell: &Cell) -> Result<HashMap<&'static str, Cell>> {
        let mut map = HashMap::new();
        let children = cell.references_count();
        match &self.kind {
            ShapeKind::Any => {
                return Ok(map)
            }
            ShapeKind::Literal(data) => {
                if cell.bit_length() != data.len() * 8 {
                    fail!("data size doesn't match")
                }
                if &cell.data()[..data.len()] != data {
                    fail!("data doesn't match")
                }
                if self.refs.len() != children {
                    fail!("number of children doesn't match")
                }
            }
            ShapeKind::Var(name) => {
                map.insert(name.clone(), cell.clone());
                return Ok(map)
            }
        }
        for i in 0..children {
            let child = &cell.reference(i).unwrap();
            map.extend(self.refs[i].captures(child)?.into_iter());
        }
        Ok(map)
    }
}
