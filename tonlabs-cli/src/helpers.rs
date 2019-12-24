use ton_client_rs::Ed25519KeyPair;
use std::io::Read;
use std::fs::File;

pub fn read_keys(filename: &str) -> Result<Ed25519KeyPair, String> {
    let mut f = File::open(filename)
        .map_err(|e| format!("failed to open keypair file: {}", e.to_string()))?;
    let mut buf = [0; 64];
    let n = f.read(&mut buf[..])
        .map_err(|e| format!("failed to read keypair from file: {}", e.to_string()))?;
    if n != 64 {
        Err("keypair file is invalid".to_string())?;
    }
    Ok(Ed25519KeyPair::zero().from_bytes(buf))
}