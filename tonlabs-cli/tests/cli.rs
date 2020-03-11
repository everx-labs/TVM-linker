use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::path::Path;
use std::process::Command;
#[test]
fn test_call_giver() -> Result<(), Box<std::error::Error>> {
    let giver_keys_name = "giver_keys.json";
    let giver_abi_name = "giver.abi.json";
    assert_eq!(
        Path::new(giver_abi_name).exists(),
        true,
        "put giver.abi.json into the root directory of tonlabs-cli"
    );
    assert_eq!(
        Path::new(giver_keys_name).exists(),
        true,
        "put giver_keys.json into the root directory of tonlabs-cli",
    );

    let mut cmd = Command::cargo_bin("tonlabs-cli")?;
    cmd.arg("call")
        .arg("--abi")
        .arg(giver_abi_name)
        .arg("--sign")
        .arg(giver_keys_name)
        .arg("0:5b168970a9c63dd5c42a6afbcf706ef652476bb8960a22e1d8a2ad148e60c0ea")
        .arg("sendTransaction")
        .arg(r#"{"dest":"0:5b168970a9c63dd5c42a6afbcf706ef652476bb8960a22e1d8a2ad148e60c0ea","value":1000000000,"bounce":false}"#);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Succeded"));

    Ok(())
}
