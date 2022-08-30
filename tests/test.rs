extern crate predicates;
extern crate assert_cmd;

use predicates::prelude::*;
use assert_cmd::prelude::*;
use std::process::Command;
use std::env;
use std::thread::sleep;
use std::time::Duration;

const BIN_NAME: &str = "tvm_linker";


#[test]
fn test_compile_lib() -> Result<(), Box<dyn std::error::Error>> {
    let contract = "tests/test_arrays.code";
    let abi = "tests/test_arrays.abi.json";
    let lib_path = "tests/test_stdlib_sol.tvm";

    let lib_var = "TVM_LINKER_LIB_PATH";
    let prev_var =  env::var_os(lib_var);
    sleep(Duration::new(1, 0));
    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.arg("compile")
        .arg(contract)
        .arg("-a")
        .arg(abi)
        .assert()
        .stdout(predicate::str::contains("Error"));

    env::set_var(lib_var, lib_path);
    sleep(Duration::new(1, 0));
    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.arg("compile")
        .arg(contract)
        .arg("-a")
        .arg(abi)
        .assert()
        .success()
        .stdout(predicate::str::contains("Saved contract to file"));

    let mut cmd = Command::cargo_bin(BIN_NAME)?;
    cmd.arg("compile")
        .arg(contract)
        .arg("-a")
        .arg(abi)
        .arg("--print_code")
        .assert()
        .success()
        .stdout(predicate::str::contains("code\":\"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"));
    
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
        .stdout(predicate::str::contains("Error: Failed to read file tests: Is a directory (os error 21)"));

    if prev_var.is_some() {
        env::set_var(lib_var, prev_var.unwrap());
    }

    Ok(())
}