/*
 * Copyright 2018-2019 TON DEV SOLUTIONS LTD.
 *
 * Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
 * this file except in compliance with the License.
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific TON DEV software governing permissions and
 * limitations under the License.
 */
use crate::config::Config;
use crate::crypto::generate_keypair_from_mnemonic;
use crate::helpers::read_keys;
use ton_client_rs::{TonClient, TonClientConfig, TonAddress, Ed25519KeyPair};
use hex;

fn load_keypair(keys: Option<String>) -> Result<Option<Ed25519KeyPair>, String> {
    match keys {
        Some(keys) => {
            let words: Vec<&str> = keys.split(' ').collect();
            if words.len() == 0 {
                let keys = read_keys(&keys)?;
                Ok(Some(keys))
            } else {
                let pair = generate_keypair_from_mnemonic(&keys)?;

                let mut buffer = [0u8; 64];
                let public_vec = hex::decode(&pair.public)
                    .map_err(|e| format!("failed to decode public key: {}", e))?;
                let private_vec = hex::decode(&pair.secret)
                    .map_err(|e| format!("failed to decode private key: {}", e))?;
                
                buffer[..32].copy_from_slice(&private_vec);
                buffer[32..].copy_from_slice(&public_vec);

                let ed25519pair = Ed25519KeyPair::zero();
                Ok(Some(ed25519pair.from_bytes(buffer)))
            }
        },
        None => Ok(None),
    }
}

pub fn call_contract(
    conf: Config,
    addr: &str,
    abi: &str,
    method: &str,
    params: &str,
    keys: Option<String>,
    local: bool,
) -> Result<(), String> {
    let ton = TonClient::new(&TonClientConfig{
        base_url: Some(conf.url.clone()),
        message_retries_count: Some(0),
        message_expiration_timeout: Some(20000),
        message_expiration_timeout_grow_factor: Some(1.5),
        message_processing_timeout: Some(20000),
        message_processing_timeout_grow_factor: Some(1.5),
        wait_for_timeout: None,
        access_key: None,
    })
    .map_err(|e| format!("failed to create tonclient: {}", e.to_string()))?;
    
    let abi = std::fs::read_to_string(abi)
        .map_err(|e| format!("failed to read ABI file: {}", e.to_string()))?;
    
    let keys = load_keypair(keys)?;
    
    let ton_addr = TonAddress::from_str(addr)
        .map_err(|e| format!("failed to parse address: {}", e.to_string()))?;


    let method_val = method.to_owned();
    let params_val = params.to_owned();

    let result = if local {
        println!("Running get-method...");
        ton.contracts.run_local(&ton_addr, None, &abi, &method_val, None, params_val.into(), None)
            .map_err(|e| format!("run failed: {}", e.to_string()))
    } else {
        println!("Calling method...");
        ton.contracts.run(&ton_addr, &abi, &method_val, None, params_val.into(), keys.as_ref())
            .map_err(|e| format!("transaction failed: {}", e.to_string()))
    };

    match result {
        Ok(val) => {
            println!("Succeded.");
            if !val.is_null() {
                println!("Result: {}", serde_json::to_string_pretty(&val).unwrap());
            }
        },
        Err(estr) => { println!("Error: {}", estr); }
    };
    Ok(())
}
