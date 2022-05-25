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

use ton_types::{Result, SliceData, fail};
use std::cmp::Ordering;
use std::ops::Not;
use num_traits::Zero;

use super::types::{Instruction, InstructionParameter, Code, OperationBehavior};
use super::handlers::Handlers;

macro_rules! create_handler_1 {
    ($func_name:ident, $opc:literal, $mnemonic:literal) => {
        pub(super) fn $func_name(slice: &mut SliceData) -> Result<Instruction> {
            let opc = slice.get_next_int(8)?;
            if opc != $opc {
                fail!("invalid opcode");
            }
            Ok(Instruction::new($mnemonic))
        }
    };
}

macro_rules! create_handler_1t {
    ($func_name:ident, $opc:literal, $mnemonic:literal) => {
        pub(super) fn $func_name<T>(slice: &mut SliceData) -> Result<Instruction>
        where T : OperationBehavior {
            let opc = slice.get_next_int(8)?;
            if opc != $opc {
                fail!("invalid opcode");
            }
            Ok(T::insn(Instruction::new($mnemonic)))
        }
    };
}

macro_rules! create_handler_2 {
    ($func_name:ident, $opc:literal, $mnemonic:literal) => {
        pub(super) fn $func_name(slice: &mut SliceData) -> Result<Instruction> {
            let opc = slice.get_next_int(16)?;
            if opc != $opc {
                fail!("invalid opcode");
            }
            Ok(Instruction::new($mnemonic))
        }
    };
}

macro_rules! create_handler_2t {
    ($func_name:ident, $opc:literal, $mnemonic:literal) => {
        pub(super) fn $func_name<T>(slice: &mut SliceData) -> Result<Instruction>
        where T : OperationBehavior {
            let opc = slice.get_next_int(16)?;
            if opc != $opc {
                fail!("invalid opcode");
            }
            Ok(T::insn(Instruction::new($mnemonic)))
        }
    };
}

macro_rules! create_handler_2r {
    ($func_name:ident, $opc:literal, $mnemonic:literal) => {
        pub(super) fn $func_name(slice: &mut SliceData) -> Result<Instruction> {
            let opc = slice.get_next_int(16)?;
            if opc != $opc {
                fail!("invalid opcode");
            }
            let mut subslice = SliceData::from(slice.reference(0)?);
            let code = load(&mut subslice)?;
            slice.shrink_references(1..);
            Ok(Instruction::new($mnemonic).with_param(InstructionParameter::Code(code)))
        }
    };
}

macro_rules! check {
    ($expr:expr) => {
        if !$expr {
            return Err(failure::err_msg(format!("check failed {}:{}", file!(), line!())))
        }
    };
}

macro_rules! check_eq {
    ($lhs:expr, $rhs:literal) => {
        if $lhs != $rhs {
            return Err(failure::err_msg(format!("check failed {}:{}", file!(), line!())))
        }
    };
}

pub(super) fn load(slice: &mut SliceData) -> Result<Code> {
    let handlers = Handlers::new_code_page_0();
    let mut code = Code::new();
    loop {
        if slice.is_empty() {
            match slice.remaining_references().cmp(&1) {
                Ordering::Less => break,
                Ordering::Equal => {
                    *slice = SliceData::from(slice.reference(0).unwrap())
                }
                Ordering::Greater => fail!("two or more remaining references")
            }
        }
        while slice.remaining_bits() > 0 {
            let handler = handlers.get_handler(&mut slice.clone())?;
            let insn = handler(slice)?;
            code.push(insn);
        }
    }
    Ok(code)
}
pub(super) fn load_unknown(_slice: &mut SliceData) -> Result<Instruction> {
    fail!("unknown opcode")
}
pub(super) fn load_setcp(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xff);
    match slice.get_next_byte() {
        Ok(0) => Ok(Instruction::new("SETCP0")),
        _ => fail!("unknown codepage")
    }
}
create_handler_2!(load_setcpx, 0xfff0, "SETCPX");
create_handler_1!(load_nop, 0x00, "NOP");
pub(super) fn load_xchg_simple(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(4)?;
    check!(opc == 0 || opc == 1);
    let i = slice.get_next_int(4)? as isize;
    match opc {
        0 => Ok(Instruction::new("XCHG").with_param(InstructionParameter::StackRegister(i))),
        1 => Ok(Instruction::new("XCHG").with_param(InstructionParameter::StackRegisterPair(1, i))),
        _ => fail!("unknown opcode")
    }
}
pub(super) fn load_xchg_std(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x10);
    let i = slice.get_next_int(4)? as isize;
    let j = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("XCHG").with_param(InstructionParameter::StackRegisterPair(i, j)))
}
pub(super) fn load_xchg_long(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x11);
    let ii = slice.get_next_int(8)? as isize;
    Ok(Instruction::new("XCHG").with_param(InstructionParameter::StackRegisterPair(0, ii)))
}
pub(super) fn load_push_simple(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(4)?;
    check_eq!(opc, 0x2);
    let i = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("PUSH").with_param(InstructionParameter::StackRegister(i)))
}
pub(super) fn load_pop_simple(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(4)?;
    check_eq!(opc, 0x3);
    let i = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("POP").with_param(InstructionParameter::StackRegister(i)))
}
pub(super) fn load_xchg3(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(4)?;
    check_eq!(opc, 0x4);
    let i = slice.get_next_int(4)? as isize;
    let j = slice.get_next_int(4)? as isize;
    let k = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("XCHG3").with_param(InstructionParameter::StackRegisterTriple(i, j, k)))
}
pub(super) fn load_xchg2(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x50);
    let i = slice.get_next_int(4)? as isize;
    let j = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("XCHG2").with_param(InstructionParameter::StackRegisterPair(i, j)))
}
pub(super) fn load_xcpu(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x51);
    let i = slice.get_next_int(4)? as isize;
    let j = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("XCPU").with_param(InstructionParameter::StackRegisterPair(i, j)))
}
pub(super) fn load_puxc(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x52);
    let i = slice.get_next_int(4)? as isize;
    let j = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("PUXC").with_param(InstructionParameter::StackRegisterPair(i, j - 1)))
}
pub(super) fn load_push2(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x53);
    let i = slice.get_next_int(4)? as isize;
    let j = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("PUSH2").with_param(InstructionParameter::StackRegisterPair(i, j)))
}
pub(super) fn load_xc2pu(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x541);
    let i = slice.get_next_int(4)? as isize;
    let j = slice.get_next_int(4)? as isize;
    let k = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("XC2PU").with_param(InstructionParameter::StackRegisterTriple(i, j, k)))
}
pub(super) fn load_xcpuxc(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x542);
    let i = slice.get_next_int(4)? as isize;
    let j = slice.get_next_int(4)? as isize;
    let k = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("XCPUXC").with_param(InstructionParameter::StackRegisterTriple(i, j, k - 1)))
}
pub(super) fn load_xcpu2(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x543);
    let i = slice.get_next_int(4)? as isize;
    let j = slice.get_next_int(4)? as isize;
    let k = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("XCPU2").with_param(InstructionParameter::StackRegisterTriple(i, j, k)))
}
pub(super) fn load_puxc2(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x544);
    let i = slice.get_next_int(4)? as isize;
    let j = slice.get_next_int(4)? as isize;
    let k = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("PUXC2").with_param(InstructionParameter::StackRegisterTriple(i, j - 1, k - 1)))
}
pub(super) fn load_puxcpu(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x545);
    let i = slice.get_next_int(4)? as isize;
    let j = slice.get_next_int(4)? as isize;
    let k = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("PUXCPU").with_param(InstructionParameter::StackRegisterTriple(i, j - 1, k - 1)))
}
pub(super) fn load_pu2xc(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x546);
    let i = slice.get_next_int(4)? as isize;
    let j = slice.get_next_int(4)? as isize;
    let k = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("PU2XC").with_param(InstructionParameter::StackRegisterTriple(i, j - 1, k - 2)))
}
pub(super) fn load_push3(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x547);
    let i = slice.get_next_int(4)? as isize;
    let j = slice.get_next_int(4)? as isize;
    let k = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("PUSH3").with_param(InstructionParameter::StackRegisterTriple(i, j, k)))
}
pub(super) fn load_blkswap(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x55);
    let i = slice.get_next_int(4)? as usize;
    let j = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("BLKSWAP").with_param(InstructionParameter::LengthAndIndex(i + 1, j + 1)))
}
pub(super) fn load_push(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x56);
    let ii = slice.get_next_int(8)? as isize;
    Ok(Instruction::new("PUSH").with_param(InstructionParameter::StackRegister(ii)))
}
pub(super) fn load_pop(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x57);
    let ii = slice.get_next_int(8)? as isize;
    Ok(Instruction::new("POP").with_param(InstructionParameter::StackRegister(ii)))
}
create_handler_1!(load_rot,    0x58, "ROT");
create_handler_1!(load_rotrev, 0x59, "ROTREV");
create_handler_1!(load_swap2,  0x5a, "SWAP2");
create_handler_1!(load_drop2,  0x5b, "DROP2");
create_handler_1!(load_dup2,   0x5c, "DUP2");
create_handler_1!(load_over2,  0x5d, "OVER2");
pub(super) fn load_reverse(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x5e);
    let i = slice.get_next_int(4)? as usize;
    let j = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("REVERSE").with_param(InstructionParameter::LengthAndIndex(i + 2, j)))
}
pub(super) fn load_blkdrop(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x5f0);
    let i = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("BLKDROP").with_param(InstructionParameter::Length(i)))
}
pub(super) fn load_blkpush(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x5f);
    let i = slice.get_next_int(4)? as usize;
    let j = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("BLKPUSH").with_param(InstructionParameter::LengthAndIndex(i, j)))
}
create_handler_1!(load_pick,     0x60, "PICK");
create_handler_1!(load_rollx,    0x61, "ROLLX");
create_handler_1!(load_rollrevx, 0x62, "ROLLREVX");
create_handler_1!(load_blkswx,   0x63, "BLKSWX");
create_handler_1!(load_revx,     0x64, "REVX");
create_handler_1!(load_dropx,    0x65, "DROPX");
create_handler_1!(load_tuck,     0x66, "TUCK");
create_handler_1!(load_xchgx,    0x67, "XCHGX");
create_handler_1!(load_depth,    0x68, "DEPTH");
create_handler_1!(load_chkdepth, 0x69, "CHKDEPTH");
create_handler_1!(load_onlytopx, 0x6a, "ONLYTOPX");
create_handler_1!(load_onlyx,    0x6b, "ONLYX");
pub(super) fn load_blkdrop2(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x6c);
    let i = slice.get_next_int(4)? as usize;
    check!(i > 0);
    let j = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("BLKDROP2").with_param(InstructionParameter::LengthAndIndex(i, j)))
}
create_handler_1!(load_null,   0x6d, "NULL");
create_handler_1!(load_isnull, 0x6e, "ISNULL");
pub(super) fn load_tuple_create(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x6f0);
    let k = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("TUPLE").with_param(InstructionParameter::Length(k)))
}
pub(super) fn load_tuple_index(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x6f1);
    let k = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("INDEX").with_param(InstructionParameter::Length(k)))
}
pub(super) fn load_tuple_un(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x6f2);
    let k = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("UNTUPLE").with_param(InstructionParameter::Length(k)))
}
pub(super) fn load_tuple_unpackfirst(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x6f3);
    let k = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("UNPACKFIRST").with_param(InstructionParameter::Length(k)))
}
pub(super) fn load_tuple_explode(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x6f4);
    let n = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("EXPLODE").with_param(InstructionParameter::Length(n)))
}
pub(super) fn load_tuple_setindex(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x6f5);
    let k = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("SETINDEX").with_param(InstructionParameter::Length(k)))
}
pub(super) fn load_tuple_index_quiet(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x6f6);
    let k = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("INDEXQ").with_param(InstructionParameter::Length(k)))
}
pub(super) fn load_tuple_setindex_quiet(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x6f7);
    let k = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("SETINDEXQ").with_param(InstructionParameter::Length(k)))
}
create_handler_2!(load_tuple_createvar,         0x6f80, "TUPLEVAR");
create_handler_2!(load_tuple_indexvar,          0x6f81, "INDEXVAR");
create_handler_2!(load_tuple_untuplevar,        0x6f82, "UNTUPLEVAR");
create_handler_2!(load_tuple_unpackfirstvar,    0x6f83, "UNPACKFIRSTVAR");
create_handler_2!(load_tuple_explodevar,        0x6f84, "EXPLODEVAR");
create_handler_2!(load_tuple_setindexvar,       0x6f85, "SETINDEXVAR");
create_handler_2!(load_tuple_indexvar_quiet,    0x6f86, "INDEXVARQ");
create_handler_2!(load_tuple_setindexvar_quiet, 0x6f87, "SETINDEXVARQ");
create_handler_2!(load_tuple_len,               0x6f88, "TLEN");
create_handler_2!(load_tuple_len_quiet,         0x6f89, "QTLEN");
create_handler_2!(load_istuple,                 0x6f8a, "ISTUPLE");
create_handler_2!(load_tuple_last,              0x6f8b, "LAST");
create_handler_2!(load_tuple_push,              0x6f8c, "TPUSH");
create_handler_2!(load_tuple_pop,               0x6f8d, "TPOP");
create_handler_2!(load_nullswapif,              0x6fa0, "NULLSWAPIF");
create_handler_2!(load_nullswapifnot,           0x6fa1, "NULLSWAPIFNOT");
create_handler_2!(load_nullrotrif,              0x6fa2, "NULLROTRIF");
create_handler_2!(load_nullrotrifnot,           0x6fa3, "NULLROTRIFNOT");
create_handler_2!(load_nullswapif2,             0x6fa4, "NULLSWAPIF2");
create_handler_2!(load_nullswapifnot2,          0x6fa5, "NULLSWAPIFNOT2");
create_handler_2!(load_nullrotrif2,             0x6fa6, "NULLROTRIF2");
create_handler_2!(load_nullrotrifnot2,          0x6fa7, "NULLROTRIFNOT2");
pub(super) fn load_tuple_index2(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0x6fb);
    let i = slice.get_next_int(2)? as isize;
    let j = slice.get_next_int(2)? as isize;
    Ok(Instruction::new("INDEX2").with_param(InstructionParameter::StackRegisterPair(i, j)))
}
pub(super) fn load_tuple_index3(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(10)?;
    check_eq!(opc << 2, 0x6fe);
    let i = slice.get_next_int(2)? as isize;
    let j = slice.get_next_int(2)? as isize;
    let k = slice.get_next_int(2)? as isize;
    Ok(Instruction::new("INDEX3").with_param(InstructionParameter::StackRegisterTriple(i, j, k)))
}
pub(super) fn load_pushint(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    assert!((0x70..0x82).contains(&opc));
    let mut x: i16 = 0;
    if opc <= 0x7a {
        x = opc as i16 - 0x70;
    } else if opc < 0x80 {
        x = opc as i16 - 0x80;
    } else if opc == 0x80 {
        x = (slice.get_next_int(8).unwrap() as i8) as i16;
    } else if opc == 0x81 {
        x = slice.get_next_int(16).unwrap() as i16;
    }
    Ok(Instruction::new("PUSHINT").with_param(InstructionParameter::Integer(x as isize)))
}
// adapted from ton-labs-vm/src/stack/integer/conversion.rs
pub(super) fn load_bigint(slice: &mut SliceData) -> Result<num::BigInt>
{
    fn twos_complement(digits: &mut Vec<u32>)
    {
        let mut carry = true;
        for d in digits {
            *d = d.not();
            if carry {
                *d = d.wrapping_add(1);
                carry = d.is_zero();
            }
        }
    }
    let first_byte = slice.get_next_byte()?;
    let byte_len = ((first_byte & 0b11111000u8) as usize >> 3) + 3;
    let greatest3bits = (first_byte & 0b111) as u32;
    let digit_count = (byte_len + 3) >> 2;
    let mut digits: Vec<u32> = vec![0; digit_count];
    let (sign, mut value) = if greatest3bits & 0b100 == 0 {
        (num::bigint::Sign::Plus, greatest3bits)
    } else {
        (num::bigint::Sign::Minus, 0xFFFF_FFF8u32 | greatest3bits)
    };

    let mut upper = byte_len & 0b11;
    if upper == 0 {
        upper = 4;
    }
    for _ in 1..upper {
        value <<= 8;
        value |= slice.get_next_byte()? as u32;
    }
    let last_index = digit_count - 1;
    digits[last_index] = value;

    for i in (0..last_index).rev() {
        let mut value = (slice.get_next_byte()? as u32) << 24;
        value |= (slice.get_next_byte()? as u32) << 16;
        value |= (slice.get_next_byte()? as u32) << 8;
        value |= slice.get_next_byte()? as u32;

        digits[i] = value;
    }

    if sign == num::bigint::Sign::Minus {
        twos_complement(&mut digits);
    }
    Ok(num::BigInt::new(sign, digits))
}
pub(super) fn load_pushint_big(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x82);
    let int = load_bigint(slice)?;
    Ok(Instruction::new("PUSHINT").with_param(InstructionParameter::BigInteger(int)))
}
pub(super) fn load_pushpow2(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x83);
    let xx = slice.get_next_int(8)? as isize;
    Ok(Instruction::new("PUSHPOW2").with_param(InstructionParameter::Integer(xx + 1)))
}
create_handler_2!(load_pushnan, 0x83ff, "PUSHNAN");
pub(super) fn load_pushpow2dec(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x84);
    let xx = slice.get_next_int(8)? as isize;
    Ok(Instruction::new("PUSHPOW2DEC").with_param(InstructionParameter::Integer(xx + 1)))
}
pub(super) fn load_pushnegpow2(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x85);
    let xx = slice.get_next_int(8)? as isize;
    Ok(Instruction::new("PUSHNEGPOW2").with_param(InstructionParameter::Integer(xx + 1)))
}
pub(super) fn load_pushref(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x88);
    slice.shrink_references(1..);
    Ok(Instruction::new("PUSHREF"))
}
pub(super) fn load_pushrefslice(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x89);
    slice.shrink_references(1..);
    Ok(Instruction::new("PUSHREFSLICE"))
}
pub(super) fn load_pushrefcont(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x8a);
    let mut subslice = SliceData::from(slice.reference(0)?);
    let code = load(&mut subslice)?;
    slice.shrink_references(1..);
    Ok(Instruction::new("PUSHREFCONT").with_param(InstructionParameter::Code(code)))
}
pub(super) fn load_pushslice_short(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x8b);
    let x = slice.get_next_int(4).unwrap() as usize;
    let mut bitstring = slice.get_next_slice(x * 8 + 4)?;
    bitstring.trim_right();
    Ok(Instruction::new("PUSHSLICE").with_param(InstructionParameter::Slice(bitstring)))
}
pub(super) fn load_pushslice_mid(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x8c);
    let r = slice.get_next_int(2)?;
    check_eq!(r, 0); // TODO
    let xx = slice.get_next_int(5).unwrap() as usize;
    let mut bitstring = slice.get_next_slice(xx * 8 + 1)?;
    bitstring.trim_right();
    Ok(Instruction::new("PUSHSLICE").with_param(InstructionParameter::Slice(bitstring)))
}
pub(super) fn load_pushslice_long(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0x8d);
    let r = slice.get_next_int(3)?;
    check_eq!(r, 0); // TODO
    let xx = slice.get_next_int(7).unwrap() as usize;
    let mut bitstring = slice.get_next_slice(xx * 8 + 6)?;
    bitstring.trim_right();
    Ok(Instruction::new("PUSHSLICE").with_param(InstructionParameter::Slice(bitstring)))
}
pub(super) fn load_pushcont_long(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(7)?;
    check_eq!(opc << 1, 0x8e);
    let r = slice.get_next_int(2).unwrap() as usize;
    let xx = slice.get_next_int(7).unwrap() as usize;
    let bits = xx * 8;

    let mut subslice = slice.clone();
    subslice.shrink_data(..bits);
    subslice.shrink_references(..r);
    let code = load(&mut subslice)?;

    slice.shrink_data(bits..);
    slice.shrink_references(r..);

    Ok(Instruction::new("PUSHCONT").with_param(InstructionParameter::Code(code)))
}
pub(super) fn load_pushcont_short(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(4)?;
    check_eq!(opc, 0x9);
    let x = slice.get_next_int(4).unwrap() as usize;
    let mut body = slice.get_next_slice(x * 8)?;
    let code = load(&mut body)?;
    Ok(Instruction::new("PUSHCONT").with_param(InstructionParameter::Code(code)))
}
create_handler_1t!(load_add,    0xa0, "ADD");
create_handler_1t!(load_sub,    0xa1, "SUB");
create_handler_1t!(load_subr,   0xa2, "SUBR");
create_handler_1t!(load_negate, 0xa3, "NEGATE");
create_handler_1t!(load_inc,    0xa4, "INC");
create_handler_1t!(load_dec,    0xa5, "DEC");
pub(super) fn load_addconst<T>(slice: &mut SliceData)  -> Result<Instruction>
where T : OperationBehavior {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xa6);
    let cc = slice.get_next_int(8).unwrap() as i8;
    Ok(T::insn(Instruction::new("ADDCONST")).with_param(InstructionParameter::Integer(cc as isize)))
}
pub(super) fn load_mulconst<T>(slice: &mut SliceData)  -> Result<Instruction>
where T : OperationBehavior {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xa7);
    let cc = slice.get_next_int(8).unwrap() as i8;
    Ok(T::insn(Instruction::new("MULCONST")).with_param(InstructionParameter::Integer(cc as isize)))
}
create_handler_1t!(load_mul, 0xa8, "MUL");
pub(super) fn load_divmod<T>(slice: &mut SliceData)  -> Result<Instruction>
where T : OperationBehavior {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xa9);
    let opc2 = slice.get_next_int(8)?;
    match opc2 {
        0x04 => Ok(T::insn(Instruction::new("DIV"))),
        0x05 => Ok(T::insn(Instruction::new("DIVR"))),
        0x06 => Ok(T::insn(Instruction::new("DIVC"))),
        0x08 => Ok(T::insn(Instruction::new("MOD"))),
        0x0c => Ok(T::insn(Instruction::new("DIVMOD"))),
        0x0d => Ok(T::insn(Instruction::new("DIVMODR"))),
        0x0e => Ok(T::insn(Instruction::new("DIVMODC"))),
        0x24 => Ok(T::insn(Instruction::new("RSHIFT"))),
        0x34 => {
            let tt = slice.get_next_int(8)? as usize;
            Ok(T::insn(Instruction::new("RSHIFT")).with_param(InstructionParameter::Length(tt + 1)))
        },
        0x38 => {
            let tt = slice.get_next_int(8)? as usize;
            Ok(T::insn(Instruction::new("MODPOW2")).with_param(InstructionParameter::Length(tt + 1)))
        },
        0x84 => Ok(T::insn(Instruction::new("MULDIV"))),
        0x85 => Ok(T::insn(Instruction::new("MULDIVR"))),
        0x8c => Ok(T::insn(Instruction::new("MULDIVMOD"))),
        0xa4 => Ok(T::insn(Instruction::new("MULRSHIFT"))),
        0xa5 => Ok(T::insn(Instruction::new("MULRSHIFTR"))),
        0xb4 => {
            let tt = slice.get_next_int(8)? as usize;
            Ok(T::insn(Instruction::new("MULRSHIFT")).with_param(InstructionParameter::Length(tt + 1)))
        },
        0xb5 => {
            let tt = slice.get_next_int(8)? as usize;
            Ok(T::insn(Instruction::new("MULRSHIFTR")).with_param(InstructionParameter::Length(tt + 1)))
        },
        0xc4 => Ok(T::insn(Instruction::new("LSHIFTDIV"))),
        0xc5 => Ok(T::insn(Instruction::new("LSHIFTDIVR"))),
        0xd4 => {
            let tt = slice.get_next_int(8)? as usize;
            Ok(T::insn(Instruction::new("LSHIFTDIV")).with_param(InstructionParameter::Length(tt + 1)))
        },
        0xd5 => {
            let tt = slice.get_next_int(8)? as usize;
            Ok(T::insn(Instruction::new("LSHIFTDIVR")).with_param(InstructionParameter::Length(tt + 1)))
        },
        _ => {
            fail!("unknown divmod kind");
        }
    }
}
pub(super) fn load_lshift<T>(slice: &mut SliceData)  -> Result<Instruction>
where T : OperationBehavior {
    let opc = slice.get_next_int(8)?;
    match opc {
        0xaa => {
            let cc = slice.get_next_int(8)? as usize;
            Ok(T::insn(Instruction::new("LSHIFT")).with_param(InstructionParameter::Length(cc + 1)))
        }
        0xac => {
            Ok(T::insn(Instruction::new("LSHIFT")))
        }
        _ => fail!("unknown lshift kind")
    }
}
pub(super) fn load_rshift<T>(slice: &mut SliceData)  -> Result<Instruction>
where T : OperationBehavior {
    let opc = slice.get_next_int(8)?;
    match opc {
        0xab => {
            let cc = slice.get_next_int(8)? as usize;
            Ok(T::insn(Instruction::new("RSHIFT")).with_param(InstructionParameter::Length(cc + 1)))
        }
        0xad => {
            Ok(T::insn(Instruction::new("RSHIFT")))
        }
        _ => fail!("unknown rshift kind")
    }
}
create_handler_1t!(load_pow2,   0xae, "POW2");
create_handler_1t!(load_and,    0xb0, "AND");
create_handler_1t!(load_or,     0xb1, "OR");
create_handler_1t!(load_xor,    0xb2, "XOR");
create_handler_1t!(load_not,    0xb3, "NOT");
pub(super) fn load_fits<T>(slice: &mut SliceData)  -> Result<Instruction>
where T : OperationBehavior {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xb4);
    let cc = slice.get_next_int(8)? as usize;
    Ok(T::insn(Instruction::new("FITS")).with_param(InstructionParameter::Length(cc + 1)))
}
pub(super) fn load_ufits<T>(slice: &mut SliceData)  -> Result<Instruction>
where T : OperationBehavior {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xb5);
    let cc = slice.get_next_int(8)? as usize;
    Ok(T::insn(Instruction::new("UFITS")).with_param(InstructionParameter::Length(cc + 1)))
}
create_handler_2t!(load_fitsx,    0xb600, "FITSX");
create_handler_2t!(load_ufitsx,   0xb601, "UFITSX");
create_handler_2t!(load_bitsize,  0xb602, "BITSIZE");
create_handler_2t!(load_ubitsize, 0xb603, "UBITSIZE");
create_handler_2t!(load_min,      0xb608, "MIN");
create_handler_2t!(load_max,      0xb609, "MAX");
create_handler_2t!(load_minmax,   0xb60a, "MINMAX");
create_handler_2t!(load_abs,      0xb60b, "ABS");
create_handler_1t!(load_sgn,     0xb8, "SGN");
create_handler_1t!(load_less,    0xb9, "LESS");
create_handler_1t!(load_equal,   0xba, "EQUAL");
create_handler_1t!(load_leq,     0xbb, "LEQ");
create_handler_1t!(load_greater, 0xbc, "GREATER");
create_handler_1t!(load_neq,     0xbd, "NEQ");
create_handler_1t!(load_geq,     0xbe, "GEQ");
create_handler_1t!(load_cmp,     0xbf, "CMP");
pub(super) fn load_eqint<T>(slice: &mut SliceData)  -> Result<Instruction>
where T : OperationBehavior {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xc0);
    let yy = (slice.get_next_int(8).unwrap() as i8) as isize;
    Ok(T::insn(Instruction::new("EQINT")).with_param(InstructionParameter::Integer(yy)))
}
pub(super) fn load_lessint<T>(slice: &mut SliceData)  -> Result<Instruction>
where T : OperationBehavior {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xc1);
    let yy = (slice.get_next_int(8).unwrap() as i8) as isize;
    Ok(T::insn(Instruction::new("LESSINT")).with_param(InstructionParameter::Integer(yy)))
}
pub(super) fn load_gtint<T>(slice: &mut SliceData)  -> Result<Instruction>
where T : OperationBehavior {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xc2);
    let yy = (slice.get_next_int(8).unwrap() as i8) as isize;
    Ok(T::insn(Instruction::new("GTINT")).with_param(InstructionParameter::Integer(yy)))
}
pub(super) fn load_neqint<T>(slice: &mut SliceData)  -> Result<Instruction>
where T : OperationBehavior {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xc3);
    let yy = (slice.get_next_int(8).unwrap() as i8) as isize;
    Ok(T::insn(Instruction::new("NEQINT")).with_param(InstructionParameter::Integer(yy)))
}
create_handler_1!(load_isnan,  0xc4, "ISNAN");
create_handler_1!(load_chknan, 0xc5, "CHKNAN");
create_handler_2!(load_sempty,      0xc700, "SEMPTY");
create_handler_2!(load_sdempty,     0xc701, "SDEMPTY");
create_handler_2!(load_srempty,     0xc702, "SREMPTY");
create_handler_2!(load_sdfirst,     0xc703, "SDFIRST");
create_handler_2!(load_sdlexcmp,    0xc704, "SDLEXCMP");
create_handler_2!(load_sdeq,        0xc705, "SDEQ");
create_handler_2!(load_sdpfx,       0xc708, "SDPFX");
create_handler_2!(load_sdpfxrev,    0xc709, "SDPFXREV");
create_handler_2!(load_sdppfx,      0xc70a, "SDPPFX");
create_handler_2!(load_sdppfxrev,   0xc70b, "SDPPFXREV");
create_handler_2!(load_sdsfx,       0xc70c, "SDSFX");
create_handler_2!(load_sdsfxrev,    0xc70d, "SDSFXREV");
create_handler_2!(load_sdpsfx,      0xc70e, "SDPSFX");
create_handler_2!(load_sdpsfxrev,   0xc70f, "SDPSFXREV");
create_handler_2!(load_sdcntlead0,  0xc710, "SDCNTLEAD0");
create_handler_2!(load_sdcntlead1,  0xc711, "SDCNTLEAD1");
create_handler_2!(load_sdcnttrail0, 0xc712, "SDCNTTRAIL0");
create_handler_2!(load_sdcnttrail1, 0xc713, "SDCNTTRAIL1");
create_handler_1!(load_newc, 0xc8, "NEWC");
create_handler_1!(load_endc, 0xc9, "ENDC");
pub(super) fn load_sti(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xca);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("STI").with_param(InstructionParameter::Length(cc + 1)))
}
pub(super) fn load_stu(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xcb);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("STU").with_param(InstructionParameter::Length(cc + 1)))
}
create_handler_1!(load_stref,   0xcc, "STREF");
create_handler_1!(load_endcst,  0xcd, "STBREFR");
create_handler_1!(load_stslice, 0xce, "STSLICE");
create_handler_2!(load_stix,   0xcf00, "STIX");
create_handler_2!(load_stux,   0xcf01, "STUX");
create_handler_2!(load_stixr,  0xcf02, "STIXR");
create_handler_2!(load_stuxr,  0xcf03, "STUXR");
create_handler_2!(load_stixq,  0xcf04, "STIXQ");
create_handler_2!(load_stuxq,  0xcf05, "STUXQ");
create_handler_2!(load_stixrq, 0xcf06, "STIXRQ");
create_handler_2!(load_stuxrq, 0xcf07, "STUXRQ");
pub(super) fn load_stir(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xcf0a);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("STIR").with_param(InstructionParameter::Length(cc + 1)))
}
pub(super) fn load_stur(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xcf0b);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("STUR").with_param(InstructionParameter::Length(cc + 1)))
}

pub(super) fn load_stiq(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xcf0c);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("STI").set_quiet().with_param(InstructionParameter::Length(cc + 1)))
}
pub(super) fn load_stuq(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xcf0d);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("STU").set_quiet().with_param(InstructionParameter::Length(cc + 1)))
}
pub(super) fn load_stirq(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xcf0e);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("STIR").set_quiet().with_param(InstructionParameter::Length(cc + 1)))
}
pub(super) fn load_sturq(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xcf0f);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("STUR").set_quiet().with_param(InstructionParameter::Length(cc + 1)))
}
create_handler_2!(load_stbref,      0xcf11, "STBREF");
create_handler_2!(load_stb,         0xcf13, "STB");
create_handler_2!(load_strefr,      0xcf14, "STREFR");
create_handler_2!(load_stslicer,    0xcf16, "STSLICER");
create_handler_2!(load_stbr,        0xcf17, "STBR");
create_handler_2!(load_strefq,      0xcf18, "STREFQ");
create_handler_2!(load_stbrefq,     0xcf19, "STBREFQ");
create_handler_2!(load_stsliceq,    0xcf1a, "STSLICEQ");
create_handler_2!(load_stbq,        0xcf1b, "STBQ");
create_handler_2!(load_strefrq,     0xcf1c, "STREFRQ");
create_handler_2!(load_stbrefrq,    0xcf1d, "STBREFRQ");
create_handler_2!(load_stslicerq,   0xcf1e, "STSLICERQ");
create_handler_2!(load_stbrq,       0xcf1f, "STBRQ");
create_handler_2!(load_strefconst,  0xcf20, "STREFCONST");
create_handler_2!(load_stref2const, 0xcf21, "STREF2CONST");
create_handler_2!(load_endxc,       0xcf23, "ENDXC");
create_handler_2!(load_stile4,      0xcf28, "STILE4");
create_handler_2!(load_stule4,      0xcf29, "STULE4");
create_handler_2!(load_stile8,      0xcf2a, "STILE8");
create_handler_2!(load_stule8,      0xcf2b, "STULE8");
create_handler_2!(load_bdepth,      0xcf30, "BDEPTH");
create_handler_2!(load_bbits,       0xcf31, "BBITS");
create_handler_2!(load_brefs,       0xcf32, "BREFS");
create_handler_2!(load_bbitrefs,    0xcf33, "BBITREFS");
create_handler_2!(load_brembits,    0xcf35, "BREMBITS");
create_handler_2!(load_bremrefs,    0xcf36, "BREMREFS");
create_handler_2!(load_brembitrefs, 0xcf37, "BREMBITREFS");
pub(super) fn load_bchkbits_short(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xcf38);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("BCHKBITS").with_param(InstructionParameter::Length(cc + 1)))
}
create_handler_2!(load_bchkbits_long, 0xcf39, "BCHKBITS");
create_handler_2!(load_bchkrefs,      0xcf3a, "BCHKREFS");
create_handler_2!(load_bchkbitrefs,   0xcf3b, "BCHKBITREFS");
pub(super) fn load_bchkbitsq_short(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xcf3c);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("BCHKBITS").set_quiet().with_param(InstructionParameter::Length(cc + 1)))
}
create_handler_2!(load_bchkbitsq_long, 0xcf3d, "BCHKBITSQ");
create_handler_2!(load_bchkrefsq,      0xcf3e, "BCHKREFSQ");
create_handler_2!(load_bchkbitrefsq,   0xcf3f, "BCHKBITREFSQ");
create_handler_2!(load_stzeroes,       0xcf40, "STZEROES");
create_handler_2!(load_stones,         0xcf41, "STONES");
create_handler_2!(load_stsame,         0xcf42, "STSAME");
pub(super) fn load_stsliceconst(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(9)?;
    check_eq!(opc << 3, 0xcf8);
    let x = slice.get_next_int(2)?;
    check_eq!(x, 0);
    let y = slice.get_next_int(3)?;
    let mut sss = slice.get_next_slice(y as usize * 8 + 2)?;
    sss.trim_right();
    Ok(Instruction::new("STSLICECONST").with_param(InstructionParameter::Slice(sss)))
}
create_handler_1!(load_ctos, 0xd0, "CTOS");
create_handler_1!(load_ends, 0xd1, "ENDS");
pub(super) fn load_ldi(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xd2);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("LDI").with_param(InstructionParameter::Length(cc + 1)))
}
pub(super) fn load_ldu(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xd3);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("LDU").with_param(InstructionParameter::Length(cc + 1)))
}
create_handler_1!(load_ldref,     0xd4, "LDREF");
create_handler_1!(load_ldrefrtos, 0xd5, "LDREFRTOS");
pub(super) fn load_ldslice(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xd6);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("LDSLICE").with_param(InstructionParameter::Length(cc + 1)))
}
create_handler_2!(load_ldix,   0xd700, "LDIX");
create_handler_2!(load_ldux,   0xd701, "LDUX");
create_handler_2!(load_pldix,  0xd702, "PLDIX");
create_handler_2!(load_pldux,  0xd703, "PLDUX");
create_handler_2!(load_ldixq,  0xd704, "LDIXQ");
create_handler_2!(load_lduxq,  0xd705, "LDUXQ");
create_handler_2!(load_pldixq, 0xd706, "PLDIXQ");
create_handler_2!(load_plduxq, 0xd707, "PLDUXQ");
pub(super) fn load_pldi(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xd70a);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("PLDI").with_param(InstructionParameter::Length(cc + 1)))
}
pub(super) fn load_pldu(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xd70b);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("PLDU").with_param(InstructionParameter::Length(cc + 1)))
}
pub(super) fn load_ldiq(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xd70c);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("LDI").set_quiet().with_param(InstructionParameter::Length(cc + 1)))
}
pub(super) fn load_lduq(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xd70d);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("LDU").set_quiet().with_param(InstructionParameter::Length(cc + 1)))
}
pub(super) fn load_pldiq(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xd70e);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("PLDI").set_quiet().with_param(InstructionParameter::Length(cc + 1)))
}
pub(super) fn load_plduq(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xd70f);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("PLDU").set_quiet().with_param(InstructionParameter::Length(cc + 1)))
}
pub(super) fn load_plduz(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(13)?;
    check_eq!(opc << 3, 0xd710);
    let c = slice.get_next_int(3)? as usize;
    Ok(Instruction::new("PLDUZ").with_param(InstructionParameter::Length(32 * (c + 1))))
}
create_handler_2!(load_ldslicex,   0xd718, "LDSLICEX");
create_handler_2!(load_pldslicex,  0xd719, "PLDSLICEX");
create_handler_2!(load_ldslicexq,  0xd71a, "LDSLICEXQ");
create_handler_2!(load_pldslicexq, 0xd71b, "PLDSLICEXQ");
pub(super) fn load_pldslice(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xd71d);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("PLDSLICE").with_param(InstructionParameter::Length(cc + 1)))
}
pub(super) fn load_ldsliceq(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xd71e);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("LDSLICE").set_quiet().with_param(InstructionParameter::Length(cc + 1)))
}
pub(super) fn load_pldsliceq(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xd71f);
    let cc = slice.get_next_int(8)? as usize;
    Ok(Instruction::new("PLDSLICE").set_quiet().with_param(InstructionParameter::Length(cc + 1)))
}
create_handler_2!(load_sdskipfirst,  0xd721, "SDSKIPFIRST");
create_handler_2!(load_sdcutlast,    0xd722, "SDCUTLAST");
create_handler_2!(load_sdskiplast,   0xd723, "SDSKIPLAST");
create_handler_2!(load_sdsubstr,     0xd724, "SDSUBSTR");
create_handler_2!(load_sdbeginsx,    0xd726, "SDBEGINSX");
create_handler_2!(load_sdbeginsxq,   0xd727, "SDBEGINSXQ");
pub(super) fn load_sdbegins(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(14)?;
    check_eq!(opc << 2, 0xd728);
    let x = slice.get_next_int(7).unwrap() as usize;
    let mut bitstring = slice.get_next_slice(8 * x + 3)?;
    bitstring.trim_right();
    Ok(Instruction::new("SDBEGINS").with_param(InstructionParameter::Slice(bitstring)))
}
pub(super) fn load_sdbeginsq(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(14)?;
    check_eq!(opc << 2, 0xd72c);
    let x = slice.get_next_int(7).unwrap() as usize;
    let mut bitstring = slice.get_next_slice(8 * x + 3)?;
    bitstring.trim_right();
    Ok(Instruction::new("SDBEGINS").set_quiet().with_param(InstructionParameter::Slice(bitstring)))
}
create_handler_2!(load_scutfirst,    0xd730, "SCUTFIRST");
create_handler_2!(load_sskipfirst,   0xd731, "SSKIPFIRST");
create_handler_2!(load_scutlast,     0xd732, "SCUTLAST");
create_handler_2!(load_sskiplast,    0xd733, "SSKIPLAST");
create_handler_2!(load_subslice,     0xd734, "SUBSLICE");
create_handler_2!(load_split,        0xd736, "SPLIT");
create_handler_2!(load_splitq,       0xd737, "SPLITQ");
create_handler_2!(load_xctos,        0xd739, "XCTOS");
create_handler_2!(load_xload,        0xd73a, "XLOAD");
create_handler_2!(load_xloadq,       0xd73b, "XLOADQ");
create_handler_2!(load_schkbits,     0xd741, "SCHKBITS");
create_handler_2!(load_schkrefs,     0xd742, "SCHKREFS");
create_handler_2!(load_schkbitrefs,  0xd743, "XCHKBITREFS");
create_handler_2!(load_schkbitsq,    0xd745, "SCHKBITSQ");
create_handler_2!(load_schkrefsq,    0xd746, "SCHKREFSQ");
create_handler_2!(load_schkbitrefsq, 0xd747, "SCHKBITREFSQ");
create_handler_2!(load_pldrefvar,    0xd748, "PLDREFVAR");
create_handler_2!(load_sbits,        0xd749, "SBITS");
create_handler_2!(load_srefs,        0xd74a, "SREFS");
create_handler_2!(load_sbitrefs,     0xd74b, "SBITREFS");
create_handler_2!(load_pldref,       0xd74c, "PLDREF");
pub(super) fn load_pldrefidx(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(14)?;
    check_eq!(opc << 2, 0xd74c);
    let n = slice.get_next_int(2)? as usize;
    Ok(Instruction::new("PLDREFIDX").with_param(InstructionParameter::Length(n)))
}
create_handler_2!(load_ldile4,       0xd750, "LDILE4");
create_handler_2!(load_ldule4,       0xd751, "LDULE4");
create_handler_2!(load_ldile8,       0xd752, "LDILE8");
create_handler_2!(load_ldule8,       0xd753, "LDULE8");
create_handler_2!(load_pldile4,      0xd754, "PLDILE4");
create_handler_2!(load_pldule4,      0xd755, "PLDULE4");
create_handler_2!(load_pldile8,      0xd756, "PLDILE8");
create_handler_2!(load_pldule8,      0xd757, "PLDULE8");
create_handler_2!(load_ldile4q,      0xd758, "LDILE4Q");
create_handler_2!(load_ldule4q,      0xd759, "LDULE4Q");
create_handler_2!(load_ldile8q,      0xd75a, "LDILE8Q");
create_handler_2!(load_ldule8q,      0xd75b, "LDULE8Q");
create_handler_2!(load_pldile4q,     0xd75c, "PLDILE4Q");
create_handler_2!(load_pldule4q,     0xd75d, "PLDULE4Q");
create_handler_2!(load_pldile8q,     0xd75e, "PLDILE8Q");
create_handler_2!(load_pldule8q,     0xd75f, "PLDULE8Q");
create_handler_2!(load_ldzeroes,     0xd760, "LDZEROES");
create_handler_2!(load_ldones,       0xd761, "LDONES");
create_handler_2!(load_ldsame,       0xd762, "LDSAME");
create_handler_2!(load_sdepth,       0xd764, "SDEPTH");
create_handler_2!(load_cdepth,       0xd765, "CDEPTH");
create_handler_1!(load_callx, 0xd8, "CALLX");
create_handler_1!(load_jmpx,  0xd9, "JMPX");
pub(super) fn load_callxargs(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    match opc {
        0xda => {
            let p = slice.get_next_int(4)? as usize;
            let r = slice.get_next_int(4)? as usize;
            Ok(Instruction::new("CALLXARGS").with_param(InstructionParameter::Pargs(p)).with_param(InstructionParameter::Rargs(r)))
        }
        0xdb => {
            let z = slice.get_next_int(4)?;
            check_eq!(z, 0);
            let p = slice.get_next_int(4)? as usize;
            Ok(Instruction::new("CALLXARGS").with_param(InstructionParameter::Pargs(p)).with_param(InstructionParameter::Rargs(usize::MAX)))
        }
        _ => fail!("unknown callxargs kind")
    }
}
pub(super) fn load_jmpxargs(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xdb1);
    let p = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("JMPXARGS").with_param(InstructionParameter::Pargs(p)))
}
pub(super) fn load_retargs(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xdb2);
    let r = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("RETARGS").with_param(InstructionParameter::Rargs(r)))
}
create_handler_2!(load_ret,      0xdb30, "RET");
create_handler_2!(load_retalt,   0xdb31, "RETALT");
create_handler_2!(load_retbool,  0xdb32, "RETBOOL");
create_handler_2!(load_callcc,   0xdb34, "CALLCC");
create_handler_2!(load_jmpxdata, 0xdb35, "JMPXDATA");
pub(super) fn load_callccargs(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc, 0xdb36);
    let p = slice.get_next_int(4)? as usize;
    let r = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("CALLCCARGS").with_param(InstructionParameter::Pargs(p)).with_param(InstructionParameter::Rargs(r)))
}
create_handler_2!(load_callxva,    0xdb38, "CALLXVARARGS");
create_handler_2!(load_retva,      0xdb39, "RETVARARGS");
create_handler_2!(load_jmpxva,     0xdb3a, "JMPXVARARGS");
create_handler_2!(load_callccva,   0xdb3b, "CALLCCVARARGS");
create_handler_2r!(load_callref,    0xdb3c, "CALLREF");
create_handler_2r!(load_jmpref,     0xdb3d, "JMPREF");
create_handler_2r!(load_jmprefdata, 0xdb3e, "JMPREFDATA");
create_handler_2!(load_retdata,    0xdb3f, "RETDATA");
create_handler_1!(load_ifret,    0xdc, "IFRET");
create_handler_1!(load_ifnotret, 0xdd, "IFNOTRET");
create_handler_1!(load_if,       0xde, "IF");
create_handler_1!(load_ifnot,    0xdf, "IFNOT");
create_handler_1!(load_ifjmp,    0xe0, "IFJMP");
create_handler_1!(load_ifnotjmp, 0xe1, "IFNOTJMP");
create_handler_1!(load_ifelse,   0xe2, "IFELSE");
create_handler_2r!(load_ifref,       0xe300, "IFREF");
create_handler_2r!(load_ifnotref,    0xe301, "IFNOTREF");
create_handler_2r!(load_ifjmpref,    0xe302, "IFJMPREF");
create_handler_2r!(load_ifnotjmpref, 0xe303, "IFNOTJMPREF");
create_handler_2!(load_condsel,      0xe304, "CONDSEL");
create_handler_2!(load_condselchk,   0xe305, "CONDSELCHK");
create_handler_2!(load_ifretalt,     0xe308, "IFRETALT");
create_handler_2!(load_ifnotretalt,  0xe309, "IFNOTRETALT");
create_handler_2r!(load_ifrefelse,      0xe30d, "IFREFELSE");
create_handler_2r!(load_ifelseref,      0xe30e, "IFELSEREF");
create_handler_2r!(load_ifrefelseref,   0xe30f, "IFREFELSEREF");
create_handler_2!(load_repeat_break,    0xe314, "REPEATBRK");
create_handler_2!(load_repeatend_break, 0xe315, "REPEATENDBRK");
create_handler_2!(load_until_break,     0xe316, "UNTILBRK");
create_handler_2!(load_untilend_break,  0xe317, "UNTILENDBRK");
create_handler_2!(load_while_break,     0xe318, "WHILEBRK");
create_handler_2!(load_whileend_break,  0xe319, "WHILEENDBRK");
create_handler_2!(load_again_break,     0xe31a, "AGAINBRK");
create_handler_2!(load_againend_break,  0xe31b, "AGAINENDBRK");
pub(super) fn load_ifbitjmp(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(15)?;
    check_eq!(opc << 1, 0xe38);
    let n = slice.get_next_int(5)? as isize;
    Ok(Instruction::new("IFBITJMP").with_param(InstructionParameter::Integer(n)))
}
pub(super) fn load_ifnbitjmp(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(15)?;
    check_eq!(opc << 1, 0xe3a);
    let n = slice.get_next_int(5)? as isize;
    Ok(Instruction::new("IFNBITJMP").with_param(InstructionParameter::Integer(n)))
}
pub(super) fn load_ifbitjmpref(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(15)?;
    check_eq!(opc << 1, 0xe3c);
    let n = slice.get_next_int(5)? as isize;
    let mut subslice = SliceData::from(slice.reference(0)?);
    let code = load(&mut subslice)?;
    slice.shrink_references(1..);
    Ok(Instruction::new("IFBITJMPREF").with_param(InstructionParameter::Integer(n)).with_param(InstructionParameter::Code(code)))
}
pub(super) fn load_ifnbitjmpref(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(15)?;
    check_eq!(opc << 1, 0xe3e);
    let n = slice.get_next_int(5)? as isize;
    let mut subslice = SliceData::from(slice.reference(0)?);
    let code = load(&mut subslice)?;
    slice.shrink_references(1..);
    Ok(Instruction::new("IFNBITJMPREF").with_param(InstructionParameter::Integer(n)).with_param(InstructionParameter::Code(code)))
}
create_handler_1!(load_repeat,    0xe4, "REPEAT");
create_handler_1!(load_repeatend, 0xe5, "REPEATEND");
create_handler_1!(load_until,     0xe6, "UNTIL");
create_handler_1!(load_untilend,  0xe7, "UNTILEND");
create_handler_1!(load_while,     0xe8, "WHILE");
create_handler_1!(load_whileend,  0xe9, "WHILEEND");
create_handler_1!(load_again,     0xea, "AGAIN");
create_handler_1!(load_againend,  0xeb, "AGAINEND");
pub(super) fn load_setcontargs(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xec);
    let r = slice.get_next_int(4)? as usize;
    let n = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("SETCONTARGS").with_param(InstructionParameter::Rargs(r)).with_param(InstructionParameter::Nargs(n)))
}
pub(super) fn load_returnargs(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xed0);
    let p = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("RETURNARGS").with_param(InstructionParameter::Pargs(p)))
}
create_handler_2!(load_returnva,  0xed10, "RETURNVARARGS");
create_handler_2!(load_setcontva, 0xed11, "SETCONTVARARGS");
create_handler_2!(load_setnumva,  0xed12, "SETNUMVARARGS");
create_handler_2!(load_bless,     0xed1e, "BLESS");
create_handler_2!(load_blessva,   0xed1f, "BLESSVARARGS");
pub(super) fn load_pushctr(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xed4);
    let i = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("PUSHCTR").with_param(InstructionParameter::ControlRegister(i)))
}
pub(super) fn load_popctr(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xed5);
    let i = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("POPCTR").with_param(InstructionParameter::ControlRegister(i)))
}
pub(super) fn load_setcontctr(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xed6);
    let i = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("SETCONTCTR").with_param(InstructionParameter::ControlRegister(i)))
}
pub(super) fn load_setretctr(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xed7);
    let i = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("SETRETCTR").with_param(InstructionParameter::ControlRegister(i)))
}
pub(super) fn load_setaltctr(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xed8);
    let i = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("SETALTCTR").with_param(InstructionParameter::ControlRegister(i)))
}
pub(super) fn load_popsave(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xed9);
    let i = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("POPSAVE").with_param(InstructionParameter::ControlRegister(i)))
}
pub(super) fn load_save(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xeda);
    let i = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("SAVE").with_param(InstructionParameter::ControlRegister(i)))
}
pub(super) fn load_savealt(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xedb);
    let i = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("SAVEALT").with_param(InstructionParameter::ControlRegister(i)))
}
pub(super) fn load_saveboth(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xedc);
    let i = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("SAVEBOTH").with_param(InstructionParameter::ControlRegister(i)))
}
create_handler_2!(load_pushctrx,     0xede0, "PUSHCTRX");
create_handler_2!(load_popctrx,      0xede1, "POPCTRX");
create_handler_2!(load_setcontctrx,  0xede2, "SETCONTCTRX");
create_handler_2!(load_compos,       0xedf0, "COMPOS");
create_handler_2!(load_composalt,    0xedf1, "COMPOSALT");
create_handler_2!(load_composboth,   0xedf2, "COMPOSBOTH");
create_handler_2!(load_atexit,       0xedf3, "ATEXIT");
create_handler_2!(load_atexitalt,    0xedf4, "ATEXITALT");
create_handler_2!(load_setexitalt,   0xedf5, "SETEXITALT");
create_handler_2!(load_thenret,      0xedf6, "THENRET");
create_handler_2!(load_thenretalt,   0xedf7, "THENRETALT");
create_handler_2!(load_invert,       0xedf8, "INVERT");
create_handler_2!(load_booleval,     0xedf9, "BOOLEVAL");
create_handler_2!(load_samealt,      0xedfa, "SAMEALT");
create_handler_2!(load_samealt_save, 0xedfb, "SAMEALTSAVE");
pub(super) fn load_blessargs(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xee);
    let r = slice.get_next_int(4)? as usize;
    let n = slice.get_next_int(4)? as isize;
    Ok(Instruction::new("BLESSARGS").with_param(InstructionParameter::Rargs(r)).with_param(InstructionParameter::Nargs(n)))
}
pub(super) fn load_call_short(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xf0);
    let n = slice.get_next_int(8)? as isize;
    Ok(Instruction::new("CALL").with_param(InstructionParameter::Nargs(n)))
}
pub(super) fn load_call_long(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(10)?;
    check_eq!(opc << 2, 0xf10);
    let n = slice.get_next_int(14)? as isize;
    Ok(Instruction::new("CALL").with_param(InstructionParameter::Nargs(n)))
}
pub(super) fn load_jmp(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(10)?;
    check_eq!(opc << 2, 0xf14);
    let n = slice.get_next_int(14)? as isize;
    Ok(Instruction::new("JMPDICT").with_param(InstructionParameter::Nargs(n)))
}
pub(super) fn load_prepare(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(10)?;
    check_eq!(opc << 2, 0xf18);
    let n = slice.get_next_int(14)? as isize;
    Ok(Instruction::new("PREPARE").with_param(InstructionParameter::Nargs(n)))
}
pub(super) fn load_throw_short(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(10)?;
    check_eq!(opc << 2, 0xf20);
    let nn = slice.get_next_int(6)? as isize;
    Ok(Instruction::new("THROW").with_param(InstructionParameter::Integer(nn)))
}
pub(super) fn load_throwif_short(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(10)?;
    check_eq!(opc << 2, 0xf24);
    let nn = slice.get_next_int(6)? as isize;
    Ok(Instruction::new("THROWIF").with_param(InstructionParameter::Integer(nn)))
}
pub(super) fn load_throwifnot_short(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(10)?;
    check_eq!(opc << 2, 0xf28);
    let nn = slice.get_next_int(6)? as isize;
    Ok(Instruction::new("THROWIFNOT").with_param(InstructionParameter::Integer(nn)))
}
pub(super) fn load_throw_long(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(13)?;
    check_eq!(opc << 3, 0xf2c0);
    let nn = slice.get_next_int(11)? as isize;
    Ok(Instruction::new("THROW").with_param(InstructionParameter::Integer(nn)))
}
pub(super) fn load_throwarg(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(13)?;
    check_eq!(opc << 3, 0xf2c8);
    let nn = slice.get_next_int(11)? as isize;
    Ok(Instruction::new("THROWARG").with_param(InstructionParameter::Integer(nn)))
}
pub(super) fn load_throwif_long(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(13)?;
    check_eq!(opc << 3, 0xf2d0);
    let nn = slice.get_next_int(11)? as isize;
    Ok(Instruction::new("THROWIF").with_param(InstructionParameter::Integer(nn)))
}
pub(super) fn load_throwargif(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(13)?;
    check_eq!(opc << 3, 0xf2d8);
    let nn = slice.get_next_int(11)? as isize;
    Ok(Instruction::new("THROWARGIF").with_param(InstructionParameter::Integer(nn)))
}
pub(super) fn load_throwifnot_long(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(13)?;
    check_eq!(opc << 3, 0xf2e0);
    let nn = slice.get_next_int(11)? as isize;
    Ok(Instruction::new("THROWIFNOT").with_param(InstructionParameter::Integer(nn)))
}
pub(super) fn load_throwargifnot(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(13)?;
    check_eq!(opc << 3, 0xf2e8);
    let nn = slice.get_next_int(11)? as isize;
    Ok(Instruction::new("THROWARGIFNOT").with_param(InstructionParameter::Integer(nn)))
}
create_handler_2!(load_throwany,         0xf2f0, "THROWANY");
create_handler_2!(load_throwargany,      0xf2f1, "THROWARGANY");
create_handler_2!(load_throwanyif,       0xf2f2, "THROWANYIF");
create_handler_2!(load_throwarganyif,    0xf2f3, "THROWARGANYIF");
create_handler_2!(load_throwanyifnot,    0xf2f4, "THROWANYIFNOT");
create_handler_2!(load_throwarganyifnot, 0xf2f5, "THROWARGANYIFNOT");
create_handler_2!(load_try,              0xf2ff, "TRY");
pub(super) fn load_tryargs(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(8)?;
    check_eq!(opc, 0xf3);
    let p = slice.get_next_int(4)? as usize;
    let r = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("TRYARGS").with_param(InstructionParameter::Pargs(p)).with_param(InstructionParameter::Rargs(r)))
}
create_handler_2!(load_ldgrams,     0xfa00, "LDGRAMS");
create_handler_2!(load_ldvarint16,  0xfa01, "LDVARINT16");
create_handler_2!(load_stgrams,     0xfa02, "STGRAMS");
create_handler_2!(load_stvarint16,  0xfa03, "STVARINT16");
create_handler_2!(load_ldvaruint32, 0xfa04, "LDVARUINT32");
create_handler_2!(load_ldvarint32,  0xfa05, "LDVARINT32");
create_handler_2!(load_stvaruint32, 0xfa06, "STVARUINT32");
create_handler_2!(load_stvarint32,  0xfa07, "STVARINT32");
pub(super) fn load_ldmsgaddr<T>(slice: &mut SliceData)  -> Result<Instruction>
where T : OperationBehavior {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc & 0xfffe, 0xfa40);
    Ok(T::insn(Instruction::new("LDMSGADDR")))
}
pub(super) fn load_parsemsgaddr<T>(slice: &mut SliceData)  -> Result<Instruction>
where T : OperationBehavior {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc & 0xfffe, 0xfa42);
    Ok(T::insn(Instruction::new("PARSEMSGADDR")))
}
pub(super) fn load_rewrite_std_addr<T>(slice: &mut SliceData)  -> Result<Instruction>
where T : OperationBehavior {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc & 0xfffe, 0xfa44);
    Ok(T::insn(Instruction::new("REWRITESTDADDR")))
}
pub(super) fn load_rewrite_var_addr<T>(slice: &mut SliceData)  -> Result<Instruction>
where T : OperationBehavior {
    let opc = slice.get_next_int(16)?;
    check_eq!(opc & 0xfffe, 0xfa46);
    Ok(T::insn(Instruction::new("REWRITEVARADDR")))
}
create_handler_2!(load_sendrawmsg,         0xfb00, "SENDRAWMSG");
create_handler_2!(load_rawreserve,         0xfb02, "RAWRESERVE");
create_handler_2!(load_rawreservex,        0xfb03, "RAWRESERVEX");
create_handler_2!(load_setcode,            0xfb04, "SETCODE");
create_handler_2!(load_setlibcode,         0xfb06, "SETLIBCODE");
create_handler_2!(load_changelib,          0xfb07, "CHANGELIB");
create_handler_2!(load_stdict,             0xf400, "STDICT");
create_handler_2!(load_skipdict,           0xf401, "SKIPDICT");
create_handler_2!(load_lddicts,            0xf402, "LDDICTS");
create_handler_2!(load_plddicts,           0xf403, "PLDDICTS");
create_handler_2!(load_lddict,             0xf404, "LDDICT");
create_handler_2!(load_plddict,            0xf405, "PLDDICT");
create_handler_2!(load_lddictq,            0xf406, "LDDICT");
create_handler_2!(load_plddictq,           0xf407, "PLDDICT");
create_handler_2!(load_dictget,            0xf40a, "DICTGET");
create_handler_2!(load_dictgetref,         0xf40b, "DICTGETREF");
create_handler_2!(load_dictiget,           0xf40c, "DICTIGET");
create_handler_2!(load_dictigetref,        0xf40d, "DICTIGETREF");
create_handler_2!(load_dictuget,           0xf40e, "DICTUGET");
create_handler_2!(load_dictugetref,        0xf40f, "DICTUGETREF");
create_handler_2!(load_dictset,            0xf412, "DICTSET");
create_handler_2!(load_dictsetref,         0xf413, "DICTSETREF");
create_handler_2!(load_dictiset,           0xf414, "DICTISET");
create_handler_2!(load_dictisetref,        0xf415, "DICTISETREF");
create_handler_2!(load_dictuset,           0xf416, "DICTUSET");
create_handler_2!(load_dictusetref,        0xf417, "DICTUSETREF");
create_handler_2!(load_dictsetget,         0xf41a, "DICTSETGET");
create_handler_2!(load_dictsetgetref,      0xf41b, "DICTSETGETREF");
create_handler_2!(load_dictisetget,        0xf41c, "DICTISETGET");
create_handler_2!(load_dictisetgetref,     0xf41d, "DICTISETGETREF");
create_handler_2!(load_dictusetget,        0xf41e, "DICTUSETGET");
create_handler_2!(load_dictusetgetref,     0xf41f, "DICTUSETGETREF");
create_handler_2!(load_dictreplace,        0xf422, "DICTREPLACE");
create_handler_2!(load_dictreplaceref,     0xf423, "DICTREPLACEREF");
create_handler_2!(load_dictireplace,       0xf424, "DICTIREPLACE");
create_handler_2!(load_dictireplaceref,    0xf425, "DICTIREPLACEREF");
create_handler_2!(load_dictureplace,       0xf426, "DICTUREPLACE");
create_handler_2!(load_dictureplaceref,    0xf427, "DICTUREPLACEREF");
create_handler_2!(load_dictreplaceget,     0xf42a, "DICTREPLACEGET");
create_handler_2!(load_dictreplacegetref,  0xf42b, "DICTREPLACEGETREF");
create_handler_2!(load_dictireplaceget,    0xf42c, "DICTIREPLACEGET");
create_handler_2!(load_dictireplacegetref, 0xf42d, "DICTIREPLACEGETREF");
create_handler_2!(load_dictureplaceget,    0xf42e, "DICTUREPLACEGET");
create_handler_2!(load_dictureplacegetref, 0xf42f, "DICTUREPLACEGETREF");
create_handler_2!(load_dictadd,            0xf432, "DICTADD");
create_handler_2!(load_dictaddref,         0xf433, "DICTADDREF");
create_handler_2!(load_dictiadd,           0xf434, "DICTIADD");
create_handler_2!(load_dictiaddref,        0xf435, "DICTIADDREF");
create_handler_2!(load_dictuadd,           0xf436, "DICTUADD");
create_handler_2!(load_dictuaddref,        0xf437, "DICTUADDREF");
create_handler_2!(load_dictaddget,         0xf43a, "DICTADDGET");
create_handler_2!(load_dictaddgetref,      0xf43b, "DICTADDGETREF");
create_handler_2!(load_dictiaddget,        0xf43c, "DICTIADDGET");
create_handler_2!(load_dictiaddgetref,     0xf43d, "DICTIADDGETREF");
create_handler_2!(load_dictuaddget,        0xf43e, "DICTUADDGET");
create_handler_2!(load_dictuaddgetref,     0xf43f, "DICTUADDGETREF");
create_handler_2!(load_dictsetb,           0xf441, "DICTSETB");
create_handler_2!(load_dictisetb,          0xf442, "DICTISETB");
create_handler_2!(load_dictusetb,          0xf443, "DICTUSETB");
create_handler_2!(load_dictsetgetb,        0xf445, "DICTSETGETB");
create_handler_2!(load_dictisetgetb,       0xf446, "DICTISETGETB");
create_handler_2!(load_dictusetgetb,       0xf447, "DICTUSETGETB");
create_handler_2!(load_dictreplaceb,       0xf449, "DICTREPLACEB");
create_handler_2!(load_dictireplaceb,      0xf44a, "DICTIREPLACEB");
create_handler_2!(load_dictureplaceb,      0xf44b, "DICTUREPLACEB");
create_handler_2!(load_dictreplacegetb,    0xf44d, "DICTREPLACEGETB");
create_handler_2!(load_dictireplacegetb,   0xf44e, "DICTIREPLACEGETB");
create_handler_2!(load_dictureplacegetb,   0xf44f, "DICTUREPLACEGETB");
create_handler_2!(load_dictaddb,           0xf451, "DICTADDB");
create_handler_2!(load_dictiaddb,          0xf452, "DICTIADDB");
create_handler_2!(load_dictuaddb,          0xf453, "DICTUADDB");
create_handler_2!(load_dictaddgetb,        0xf455, "DICTADDGETB");
create_handler_2!(load_dictiaddgetb,       0xf456, "DICTIADDGETB");
create_handler_2!(load_dictuaddgetb,       0xf457, "DICTUADDGETB");
create_handler_2!(load_dictdel,            0xf459, "DICTDEL");
create_handler_2!(load_dictidel,           0xf45a, "DICTIDEL");
create_handler_2!(load_dictudel,           0xf45b, "DICTUDEL");
create_handler_2!(load_dictdelget,         0xf462, "DICTDELGET");
create_handler_2!(load_dictdelgetref,      0xf443, "DICTDELGETREF");
create_handler_2!(load_dictidelget,        0xf444, "DICTIDELGET");
create_handler_2!(load_dictidelgetref,     0xf445, "DICTIDELGETREF");
create_handler_2!(load_dictudelget,        0xf466, "DICTUDELGET");
create_handler_2!(load_dictudelgetref,     0xf467, "DICTUDELGETREF");
create_handler_2!(load_dictgetoptref,      0xf469, "DICTGETOPTREF");
create_handler_2!(load_dictigetoptref,     0xf46a, "DICTIGETOPTREF");
create_handler_2!(load_dictugetoptref,     0xf46b, "DICTUGETOPTREF");
create_handler_2!(load_dictsetgetoptref,   0xf46d, "DICTSETGETOPTREF");
create_handler_2!(load_dictisetgetoptref,  0xf46e, "DICTISETGETOPTREF");
create_handler_2!(load_dictusetgetoptref,  0xf46f, "DICTUSETGETOPTREF");
create_handler_2!(load_pfxdictset,         0xf470, "PFXDICTSET");
create_handler_2!(load_pfxdictreplace,     0xf471, "PFXDICTREPLACE");
create_handler_2!(load_pfxdictadd,         0xf472, "PFXDICTADD");
create_handler_2!(load_pfxdictdel,         0xf473, "PFXDICTDEL");
create_handler_2!(load_dictgetnext,        0xf474, "DICTGETNEXT");
create_handler_2!(load_dictgetnexteq,      0xf475, "DICTGETNEXTEQ");
create_handler_2!(load_dictgetprev,        0xf476, "DICTGETPREV");
create_handler_2!(load_dictgetpreveq,      0xf477, "DICTGETPREVEQ");
create_handler_2!(load_dictigetnext,       0xf478, "DICTIGETNEXT");
create_handler_2!(load_dictigetnexteq,     0xf479, "DICTIGETNEXTEQ");
create_handler_2!(load_dictigetprev,       0xf47a, "DICTIGETPREV");
create_handler_2!(load_dictigetpreveq,     0xf47b, "DICTIGETPREVEQ");
create_handler_2!(load_dictugetnext,       0xf47c, "DICTUGETNEXT");
create_handler_2!(load_dictugetnexteq,     0xf47d, "DICTUGETNEXTEQ");
create_handler_2!(load_dictugetprev,       0xf47e, "DICTUGETPREV");
create_handler_2!(load_dictugetpreveq,     0xf47f, "DICTUGETPREVEQ");
create_handler_2!(load_dictmin,            0xf482, "DICTMIN");
create_handler_2!(load_dictminref,         0xf483, "DICTMINREF");
create_handler_2!(load_dictimin,           0xf484, "DICTIMIN");
create_handler_2!(load_dictiminref,        0xf485, "DICTIMINREF");
create_handler_2!(load_dictumin,           0xf486, "DICTUMIN");
create_handler_2!(load_dictuminref,        0xf487, "DICTUMINREF");
create_handler_2!(load_dictmax,            0xf48a, "DICTMAX");
create_handler_2!(load_dictmaxref,         0xf48b, "DICTMAXREF");
create_handler_2!(load_dictimax,           0xf48c, "DICTIMAX");
create_handler_2!(load_dictimaxref,        0xf48d, "DICTIMAXREF");
create_handler_2!(load_dictumax,           0xf48e, "DICTUMAX");
create_handler_2!(load_dictumaxref,        0xf48f, "DICTUMAXREF");
create_handler_2!(load_dictremmin,         0xf492, "DICTREMMIN");
create_handler_2!(load_dictremminref,      0xf493, "DICTREMMINREF");
create_handler_2!(load_dictiremmin,        0xf494, "DICTIREMMIN");
create_handler_2!(load_dictiremminref,     0xf495, "DICTIREMMINREF");
create_handler_2!(load_dicturemmin,        0xf496, "DICTUREMMIN");
create_handler_2!(load_dicturemminref,     0xf497, "DICTUREMMINREF");
create_handler_2!(load_dictremmax,         0xf49a, "DICTREMMAX");
create_handler_2!(load_dictremmaxref,      0xf49b, "DICTREMMAXREF");
create_handler_2!(load_dictiremmax,        0xf49c, "DICTIREMMAX");
create_handler_2!(load_dictiremmaxref,     0xf49d, "DICTIREMMAXREF");
create_handler_2!(load_dicturemmax,        0xf49e, "DICTUREMMAX");
create_handler_2!(load_dicturemmaxref,     0xf49f, "DICTUREMMAXREF");
create_handler_2!(load_dictigetjmp,        0xf4a0, "DICTIGETJMP");
create_handler_2!(load_dictugetjmp,        0xf4a1, "DICTUGETJMP");
create_handler_2!(load_dictigetexec,       0xf4a2, "DICTIGETEXEC");
create_handler_2!(load_dictugetexec,       0xf4a3, "DICTUGETEXEC");
pub(super) fn load_dictpushconst(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(14)?;
    check_eq!(opc << 2, 0xf4a4);
    let n = slice.get_next_int(10)? as usize;
    let subslice = SliceData::from(slice.reference(0)?);
    slice.shrink_references(1..);
    Ok(Instruction::new("DICTPUSHCONST").with_param(InstructionParameter::Slice(subslice)).with_param(InstructionParameter::Length(n)))
}
create_handler_2!(load_pfxdictgetq,    0xf4a8, "PFXDICTGETQ");
create_handler_2!(load_pfxdictget,     0xf4a9, "PFXDICTGET");
create_handler_2!(load_pfxdictgetjmp,  0xf4aa, "PFXDICTGETJMP");
create_handler_2!(load_pfxdictgetexec, 0xf4ab, "PFXDICTGETEXEC");
pub(super) fn load_pfxdictswitch(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(14)?;
    check_eq!(opc << 2, 0xf4ac);
    let n = slice.get_next_int(10)? as usize;
    let subslice = SliceData::from(slice.reference(0)?);
    slice.shrink_references(1..);
    Ok(Instruction::new("PFXDICTSWITCH").with_param(InstructionParameter::Slice(subslice)).with_param(InstructionParameter::Length(n)))
}
create_handler_2!(load_subdictget,    0xf4b1, "SUBDICTGET");
create_handler_2!(load_subdictiget,   0xf4b2, "SUBDICTIGET");
create_handler_2!(load_subdictuget,   0xf4b3, "SUBDICTUGET");
create_handler_2!(load_subdictrpget,  0xf4b5, "SUBDICTRPGET");
create_handler_2!(load_subdictirpget, 0xf4b6, "SUBDICTIRPGET");
create_handler_2!(load_subdicturpget, 0xf4b7, "SUBDICTURPGET");
create_handler_2!(load_dictigetjmpz,  0xf4bc, "DICTIGETJMPZ");
create_handler_2!(load_dictugetjmpz,  0xf4bd, "DICTUGETJMPZ");
create_handler_2!(load_dictigetexecz, 0xf4be, "DICTIGETEXECZ");
create_handler_2!(load_dictugetexecz, 0xf4bf, "DICTUGETEXECZ");
create_handler_2!(load_accept,        0xf800, "ACCEPT");
create_handler_2!(load_setgaslimit,   0xf801, "SETGASLIMIT");
create_handler_2!(load_buygas,        0xf802, "BUYGAS");
create_handler_2!(load_gramtogas,     0xf804, "GRAMTOGAS");
create_handler_2!(load_gastogram,     0xf805, "GASTOGRAM");
create_handler_2!(load_commit,        0xf80f, "COMMIT");
create_handler_2!(load_randu256,      0xf810, "RANDU256");
create_handler_2!(load_rand,          0xf811, "RAND");
create_handler_2!(load_setrand,       0xf814, "SETRAND");
create_handler_2!(load_addrand,       0xf815, "ADDRAND");
pub(super) fn load_getparam(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xf82);
    let i = slice.get_next_int(4)? as usize;
    Ok(Instruction::new("GETPARAM").with_param(InstructionParameter::Length(i)))
}
create_handler_2!(load_now,              0xf823, "NOW");
create_handler_2!(load_blocklt,          0xf824, "BLOCKLT");
create_handler_2!(load_ltime,            0xf825, "LTIME");
create_handler_2!(load_randseed,         0xf826, "RANDSEED");
create_handler_2!(load_balance,          0xf827, "BALANCE");
create_handler_2!(load_my_addr,          0xf828, "MYADDR");
create_handler_2!(load_config_root,      0xf829, "CONFIGROOT");
create_handler_2!(load_config_dict,      0xf830, "CONFIGDICT");
create_handler_2!(load_config_ref_param, 0xf832, "CONFIGPARAM");
create_handler_2!(load_config_opt_param, 0xf833, "CONFIGOPTPARAM");
create_handler_2!(load_getglobvar,       0xf840, "GETGLOBVAR");
pub(super) fn load_getglob(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(11)?;
    check_eq!(opc << 1, 0xf84);
    let k = slice.get_next_int(5)? as usize;
    assert!(k != 0);
    Ok(Instruction::new("GETGLOB").with_param(InstructionParameter::Length(k)))
}
create_handler_2!(load_setglobvar, 0xf860, "SETGLOBVAR");
pub(super) fn load_setglob(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(11)?;
    check_eq!(opc << 1, 0xf86);
    let k = slice.get_next_int(5)? as usize;
    assert!(k != 0);
    Ok(Instruction::new("SETGLOB").with_param(InstructionParameter::Length(k)))
}
create_handler_2!(load_hashcu,     0xf900, "HASHCU");
create_handler_2!(load_hashsu,     0xf901, "HASHSU");
create_handler_2!(load_sha256u,    0xf902, "SHA256U");
create_handler_2!(load_chksignu,   0xf910, "CHKSIGNU");
create_handler_2!(load_chksigns,   0xf911, "CHKSIGNS");
create_handler_2!(load_cdatasizeq, 0xf940, "CDATASIZEQ");
create_handler_2!(load_cdatasize,  0xf941, "CDATASIZE");
create_handler_2!(load_sdatasizeq, 0xf942, "SDATASIZEQ");
create_handler_2!(load_sdatasize,  0xf943, "SDATASIZE");
create_handler_2!(load_dump_stack, 0xfe00, "DUMPSTK");
pub(super) fn load_dump_stack_top(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xfe0);
    let n = slice.get_next_int(4)? as isize;
    assert!(n > 0);
    Ok(Instruction::new("DUMPSTKTOP").with_param(InstructionParameter::Integer(n)))
}
create_handler_2!(load_dump_hex,  0xfe10, "HEXDUMP");
create_handler_2!(load_print_hex, 0xfe11, "HEXPRINT");
create_handler_2!(load_dump_bin,  0xfe12, "BINDUMP");
create_handler_2!(load_print_bin, 0xfe13, "BINPRINT");
create_handler_2!(load_dump_str,  0xfe14, "STRDUMP");
create_handler_2!(load_print_str, 0xfe15, "STRPRINT");
create_handler_2!(load_debug_off, 0xfe1e, "DEBUGOFF");
create_handler_2!(load_debug_on,  0xfe1f, "DEBUGON");
pub(super) fn load_dump_var(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xfe2);
    let n = slice.get_next_int(4)? as isize;
    assert!(n < 15);
    Ok(Instruction::new("DUMP").with_param(InstructionParameter::Integer(n)))
}
pub(super) fn load_print_var(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xfe3);
    let n = slice.get_next_int(4)? as isize;
    assert!(n < 15);
    Ok(Instruction::new("PRINT").with_param(InstructionParameter::Integer(n)))
}
pub(super) fn load_dump_string(slice: &mut SliceData) -> Result<Instruction> {
    let opc = slice.get_next_int(12)?;
    check_eq!(opc, 0xfef);
    let n = slice.get_next_int(4)?;
    let mode = slice.get_next_int(8)?;
    match n {
        0 => {
            check_eq!(mode, 0x00);
            Ok(Instruction::new("LOGFLUSH"))
        }
        _ => {
            if mode == 0x00 {
                let s = slice.get_next_slice(n as usize * 8)?;
                Ok(Instruction::new("LOGSTR").with_param(InstructionParameter::Slice(s)))
            } else if mode == 0x01 {
                let s = slice.get_next_slice(n as usize * 8)?;
                Ok(Instruction::new("PRINTSTR").with_param(InstructionParameter::Slice(s)))
            } else {
                fail!("unknown dump_string mode")
            }
        }
    }
}

pub fn print_code(code: &Code, indent: &str) -> String {
    let mut disasm = String::new();
    for insn in code {
        disasm += indent;
        disasm += insn.name();
        if insn.is_quiet() {
            disasm += "Q";
        }
        let len = insn.params().len();
        if len > 0 {
            disasm += " ";
        }
        for (index, param) in insn.params().iter().enumerate() {
            let last = len == (index + 1);
            match param {
                InstructionParameter::BigInteger(i) => {
                    disasm += format!("{}", i).as_str();
                }
                InstructionParameter::ControlRegister(c) => {
                    disasm += format!("c{}", c).as_str();
                }
                //InstructionParameter::DivisionMode(_) => {
                //    todo!()
                //}
                InstructionParameter::Integer(i) => {
                    disasm += format!("{}", i).as_str();
                }
                InstructionParameter::Length(l) => {
                    disasm += format!("{}", l).as_str();
                }
                InstructionParameter::LengthAndIndex(l, i) => {
                    disasm += format!("{}, {}", l, i).as_str();
                }
                InstructionParameter::Nargs(n) => {
                    disasm += format!("{}", n).as_str();
                }
                InstructionParameter::Pargs(p) => {
                    disasm += format!("{}", p).as_str();
                }
                InstructionParameter::Rargs(r) => {
                    disasm += format!("{}", r).as_str();
                }
                InstructionParameter::Slice(s) => {
                    disasm += format!("x{}", s.to_hex_string()).as_str();
                }
                InstructionParameter::StackRegister(r) => {
                    disasm += format!("s{}", r).as_str();
                }
                InstructionParameter::StackRegisterPair(ra, rb) => {
                    disasm += format!("s{}, s{}", ra, rb).as_str();
                }
                InstructionParameter::StackRegisterTriple(ra, rb, rc) => {
                    disasm += format!("s{}, s{}, s{}", ra, rb, rc).as_str();
                }
                InstructionParameter::Code(code) => {
                    assert!(last, "code param isn't last");
                    disasm += "{\n";
                    let inner_indent = String::from("  ") + indent;
                    disasm += &print_code(code, inner_indent.as_str());
                    disasm += indent;
                    disasm += "}";
                }
            }
            if !last {
                disasm += ", ";
            }
        }
        disasm += "\n";
    }
    disasm
}
