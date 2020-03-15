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
        .arg("0:2e0d054dfe43198d971c0f8eaa5f98ca8d08928ecb48a362a900997faecff2e5")
        .arg("grant")
        .arg(r#"{"addr":"0:2e0d054dfe43198d971c0f8eaa5f98ca8d08928ecb48a362a900997faecff2e5"}"#);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Succeded"));

    Ok(())
}
