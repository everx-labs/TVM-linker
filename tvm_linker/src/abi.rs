use abi_json::json_abi::encode_function_call;
use abi_json::Contract;
use ed25519_dalek::Keypair;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use tvm::stack::BuilderData;

pub fn build_abi_body(
    abi_file: &str,
    method: &str,
    params: &str,
    keypair: Option<Keypair>,
    internal: bool,
) -> Result<BuilderData, String> {
    let mut abi_json = String::new();
    let mut file = File::open(abi_file).map_err(|e| format!("cannot open abi file: {}", e))?;
    file.read_to_string(&mut abi_json).map_err(|e| format!("failed to read abi file: {}", e))?;
    
    encode_function_call(
        abi_json,
        method.to_owned(),
        params.to_owned(),
        internal,
        keypair.as_ref(),
    ).map_err(|e| format!("cannot encode abi body: {:?}", e))
}

pub fn gen_abi_id(mut abi: Option<Contract>, func_name: &str) -> u32 {
    if let Some(ref mut contract) = abi {
        let functions = contract.functions();
        let events = contract.events();
        functions.get(func_name).map(|f| f.get_input_id())           
            .or_else(|| events.get(func_name).map(|e| e.get_function_id()))
            .unwrap_or(calc_func_id(func_name))
    } else {
        calc_func_id(func_name)
    }
}


fn calc_func_id(func_interface: &str) -> u32 {
    let mut hasher = Sha256::new();
    hasher.input(func_interface.as_bytes());
    let mut id_bytes = [0u8; 4];
    id_bytes.copy_from_slice(&hasher.result()[..4]);
    u32::from_be_bytes(id_bytes)
}