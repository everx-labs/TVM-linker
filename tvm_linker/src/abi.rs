use abi_json::json_abi::encode_function_call;
use abi_json::Contract;
use ed25519_dalek::Keypair;
use sha2::{Digest, Sha256};
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

pub fn gen_abi_id(mut abi: Option<Contract>, func_name: &str) -> u32 {
    let signature = 
    if let Some(ref mut contract) = abi {
        let mut functions = contract.functions();
        functions.find(|f| f.name == func_name)
            .and_then(|f| Some(f.get_function_signature()))
            .unwrap_or(func_name.to_string())
    } else {
        func_name.to_string()
    };
    
    calc_func_id(&signature)
}


fn calc_func_id(func_interface: &str) -> u32 {
    let mut hasher = Sha256::new();
    hasher.input(func_interface.as_bytes());
    let mut id_bytes = [0u8; 4];
    id_bytes.copy_from_slice(&hasher.result()[..4]);
    u32::from_be_bytes(id_bytes)
} 

#[cfg(test)]
mod tests {
    use hex;
    use keyman::*;
    use super::*;

    #[test]
    fn test_build_abi_body() {
        let body = build_abi_body(
            "./tests/abi.abi", 
            "transfer", 
            "{\"to\":\"0x55\", \"a\":\"0x11223344\"}",
            None
        ).unwrap();
        let etalon_body: [u8; 10] = [0x00,0x15,0xFE,0xCE,0x26,0x55,0x11,0x22,0x33,0x44];
        println!("body = {}", body);
        assert_eq!(body, BuilderData::with_raw(etalon_body.to_vec(), 10*8));
    }

    #[test]
    fn test_build_abi_signed_body() {
        let pair = KeypairManager::from_secret_file("./key1").drain();
        let body = build_abi_body(
            "./tests/abi.abi", 
            "method_authorized", 
            "{\"amount\":\"0x100\"}",
            Some(pair),
        ).unwrap();
        let etalon_body: [u8; 13] = [0x00,0x97,0xCE,0x24,0xE7,0x00,0x00,0x00,0x00,0x00,0x00,0x01,0x00];
        let sign = hex::decode(
            "2915D548D7A0C7A26B89DA2E1DDAB2BAC2F2B9EA25A7F5276FFD8840F5EBF262C717B23F28E1E5AB16B973F7F55AA214C1A626130C632FDA7578F957270DF003"
        ).unwrap();
        let mut result_builder = BuilderData::with_raw(etalon_body.to_vec(), 13*8);
        let sign_builder = BuilderData::with_raw(sign, 512);
        result_builder.append_reference(sign_builder);
        println!("body = {}", body);
        assert_eq!(body, result_builder);
    }

    #[test]
    fn test_abi_ids() {
        let mut abi_str = String::new();
        let mut abi_file = File::open("./tests/piggy.abi").unwrap();
        abi_file.read_to_string(&mut abi_str).unwrap();
        let abi = Contract::load(abi_str.as_bytes()).unwrap();
        assert_eq!(gen_abi_id(Some(abi.clone()), "getGoal"), 0x4F643894);
        assert_eq!(gen_abi_id(Some(abi.clone()), "getTargetAmount"), 0x04503E90);
        assert_eq!(gen_abi_id(Some(abi.clone()), "transfer"), 0x1B2C23D4);
    }
}