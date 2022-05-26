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

#[test]
fn test_compile_lib_error() -> Result<(), Box<dyn std::error::Error>> {
    let contract = "tests/test_arrays.code";
    let abi = "tests/test_arrays.abi.json";
    let lib_path = "tests/";

    let lib_var = "TVM_LINKER_LIB_PATH";
    let prev_var =  env::var_os(lib_var);
    env::set_var(lib_var, lib_path);

    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.arg("compile")
        .arg(contract)
        .arg("-a")
        .arg(abi)
        .assert()
        .failure()
        .stdout(predicate::str::contains("qwerty"));

    if prev_var.is_some() {
        env::set_var(lib_var, prev_var.unwrap());
    }

    Ok(())
}