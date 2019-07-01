use abi_json::json_abi::encode_function_call;
use ed25519_dalek::Keypair;
use std::fs::File;
use std::io::Read;
use tvm::stack::BuilderData;

pub fn build_abi_body(abi_file: &str, method: &str, params: &str, keypair: Option<Keypair>) -> Result<BuilderData, String> {
    let mut abi_json = String::new();
    let mut file = File::open(abi_file).map_err(|e| format!("cannot open abi file: {}", e))?;
    file.read_to_string(&mut abi_json).map_err(|e| format!("failed to read abi file: {}", e))?;
    encode_function_call(
        abi_json, 
        method.to_owned(), 
        params.to_owned(),
        keypair.as_ref(),
    ).map_err(|e| format!("cannot encode abi body: {:?}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_build_abi_body() {
        let body = build_abi_body(
            "./tests/contract.abi", 
            "transfer", 
            "{\"to\":\"0x55\", \"a\":\"0x11223344\"}",
            None
        ).unwrap();
        let etalon_body: [u8; 10] = [0x00,0x15,0xFE,0xCE,0x26,0x55,0x11,0x22,0x33,0x44];
        println!("body = {}", body);
        assert_eq!(body, BuilderData::with_raw(etalon_body.to_vec(), 10*8));
    }
}