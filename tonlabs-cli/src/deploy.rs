use crate::config::Config;
use crate::helpers::read_keys;
use ton_client_rs::TonClient;

pub fn deploy_contract(conf: Config, tvc: &str, abi: &str, params: &str, keys_file: &str) -> Result<(), String> {
    let ton = TonClient::new_with_base_url(&conf.url)
        .map_err(|e| format!("failed to create tonclient: {}", e.to_string()))?;
    
    let abi = std::fs::read_to_string(abi)
        .map_err(|e| format!("failed to read ABI file: {}", e.to_string()))?;
    
    let keys = read_keys(keys_file)?;
    
    let contract = std::fs::read(tvc)
        .map_err(|e| format!("failed to read smart contract file: {}", e.to_string()))?;
    
    println!("Deploying...");
    let address = ton.contracts.deploy(&abi, &contract, params.into(), &keys)
        .map_err(|e| format!("deploy failed: {}", e.to_string()))?;

    println!("Transaction succeded.");
    println!("Contract deployed at address: {}", address);
    Ok(())
}