/*
* Copyright (C) 2019-2021 TON Labs. All Rights Reserved.
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

use ed25519_dalek::PublicKey;
use program::save_to_file;
use std::fs::OpenOptions;
use ton_sdk;
use abi::load_abi_json_string;

pub fn set_initial_data(tvc: &str, pubkey: Option<[u8; 32]>, data: &str, abi: &str) -> Result<(), String> {
    let mut state_init = OpenOptions::new().read(true).open(tvc)
        .map_err(|e| format!("unable to open contract file {}: {}", tvc, e))?;
    let abi = load_abi_json_string(abi)?;

    let mut contract_image = if let Some(key_bytes) = pubkey {
        let pubkey_object = PublicKey::from_bytes(&key_bytes)
            .map_err(|e| format!("unable to load public key: {}", e))?;
        ton_sdk::ContractImage::from_state_init_and_key(&mut state_init, &pubkey_object)
            .map_err(|e| format!("unable to load contract image: {}", e))?
    } else {
        ton_sdk::ContractImage::from_state_init(&mut state_init)
            .map_err(|e| format!("unable to load contract image: {}", e))?
    };

    contract_image.update_data(data, &abi)
        .map_err(|e| format!("unable to update contract image data: {}", e))?;

    save_to_file(contract_image.state_init(), None, 0)?;
    Ok(())
}
