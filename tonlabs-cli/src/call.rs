/*
 * Copyright 2018-2019 TON DEV SOLUTIONS LTD.
 *
 * Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
 * this file except in compliance with the License.  You may obtain a copy of the
 * License at: https://ton.dev/licenses
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific TON DEV software governing permissions and
 * limitations under the License.
 */
use crate::config::Config;
use crate::helpers::read_keys;
use ton_client_rs::{TonClient, TonAddress};

pub fn call_contract(
    conf: Config,
    addr: &str,
    abi: &str,
    method: &str,
    params: &str,
    keys_file: Option<String>,
    local: bool,
) -> Result<(), String> {
    let ton = TonClient::new_with_base_url(&conf.url)
        .map_err(|e| format!("failed to create tonclient: {}", e.to_string()))?;
    
    let abi = std::fs::read_to_string(abi)
        .map_err(|e| format!("failed to read ABI file: {}", e.to_string()))?;
    
    let keys = match keys_file {
        Some(filename) => Some(read_keys(&filename)?),
        None => None,
    };
    
    let ton_addr = TonAddress::from_str(addr)
        .map_err(|e| format!("failed to parse address: {}", e.to_string()))?;
    
    let result = if local {
        println!("Running get-method...");
        ton.contracts.run_local(&ton_addr, None, &abi, method, params.into(), None)
            .map_err(|e| format!("run failed: {}", e.to_string()))?
    } else {
        println!("Waiting...");
        ton.contracts.run(&ton_addr, &abi, method, params.into(), keys.as_ref())
            .map_err(|e| format!("transaction failed: {}", e.to_string()))?
    };

    println!("Succeded.");
    if !result.is_null() {
        println!("Result = {}", result);
    }
    Ok(())
}