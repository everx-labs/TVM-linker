/*
* Copyright (C) 2019-2021 TON Labs. All Rights Reserved.
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

extern crate predicates;
extern crate assert_cmd;

use predicates::prelude::*;
use assert_cmd::prelude::*;
use std::process::Command;
use std::env;

const BIN_NAME: &str = "tvm_linker";


#[test]
fn test_compile_lib() -> Result<(), Box<dyn std::error::Error>> {
    let contract = "tests/test_arrays.code";
    let abi = "tests/test_arrays.abi.json";
    let lib_path = "tests/test_stdlib_sol.tvm";

    let lib_var = "TVM_LINKER_LIB_PATH";
    let prev_var =  env::var_os(lib_var);

    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.arg("compile")
        .arg(contract)
        .arg("-a")
        .arg(abi)
        .assert()
        .stdout(predicate::str::contains("Error"));

    env::set_var(lib_var, lib_path);

    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.arg("compile")
        .arg(contract)
        .arg("-a")
        .arg(abi)
        .assert()
        .success()
        .stdout(predicate::str::contains("Saved contract to file"));
    
    if prev_var.is_some() {
        env::set_var(lib_var, prev_var.unwrap());
    }
    
    Ok(())
}