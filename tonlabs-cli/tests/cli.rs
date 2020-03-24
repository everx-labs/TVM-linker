use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_call_giver() -> Result<(), Box<dyn std::error::Error>> {
    let giver_abi_name = "Garant100.abi";
    let mut cmd = Command::cargo_bin("tonlabs-cli")?;
    cmd.arg("call")
        .arg("--abi")
        .arg(giver_abi_name)
        .arg("0:4e533d33972ae0cf29c560e1ef4dab7d437cddbf3f9bc0930a170db029f663f6")
        .arg("grant")
        .arg(r#"{"addr":"0:4e533d33972ae0cf29c560e1ef4dab7d437cddbf3f9bc0930a170db029f663f6"}"#);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Succeded"));

    Ok(())
}

#[test]
fn test_genaddr() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("tonlabs-cli")?;
    cmd.arg("genaddr")
        .arg("tests/samples/wallet.tvc")
        .arg("--genkey")
        .arg("tests/samples/wallet.keys.json")
        .arg("--verbose");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Input arguments:"))
        .stdout(predicate::str::contains("tvc:"))
        .stdout(predicate::str::contains("wc:"))
        .stdout(predicate::str::contains("keys:"))
        .stdout(predicate::str::contains("Raw address"))
        .stdout(predicate::str::contains("Succeded"));

        Ok(())
}

#[test]
fn test_genaddr_initdata() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("tonlabs-cli")?;
    cmd.arg("genaddr")
        .arg("tests/data.tvc")
        .arg("tests/data.abi.json")
        .arg("--genkey")
        .arg("key1")
        .arg("--save")
        .arg("--data")
        .arg(r#"{"m_owner":"0x2e0d054dfe43198d971c0f8eaa5f98ca8d08928ecb48a362a900997faecff2e5"}"#);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("TVC file updated"))
        .stdout(predicate::str::contains("Succeded"));

    Ok(())
}