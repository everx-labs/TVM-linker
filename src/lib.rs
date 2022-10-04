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

extern crate base64;
extern crate clap;
extern crate crc;
extern crate ed25519;
extern crate ed25519_dalek;
extern crate failure;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate rand;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate sha2;
extern crate simplelog;

extern crate ton_abi as abi_json;
extern crate ton_block;
extern crate ton_labs_assembler;
extern crate ton_types;
#[macro_use]
extern crate ton_vm;

pub mod abi;
pub mod keyman;
pub mod parser;
pub mod printer;
pub mod program;
pub mod resolver;
pub mod methdict;
pub mod testcall;
