/*
 * Copyright 2018-2022 TON DEV SOLUTIONS LTD.
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
use failure::format_err;
use ton_types::{Result};
use serde::Deserialize;

pub struct KeypairManager(ed25519_dalek::Keypair);

impl KeypairManager {
    pub fn from_file(filename: &str) -> Result<Self> {
        let keys_str = std::fs::read_to_string(filename)
            .map_err(|e| format_err!("failed to read the keypair file: {}", e))?;
        #[derive(Deserialize)]
        struct KeyPair {
            pub public: String,
            pub secret: String,
        }
        let keys: KeyPair = serde_json::from_str(&keys_str)
            .map_err(|e| format_err!("failed to load keypair: {}", e))?;
        let mut keypair = hex::decode(keys.secret)
            .map_err(|e| format_err!("failed to decode private key: {}", e))?;
        let mut public = hex::decode(keys.public)
            .map_err(|e| format_err!("failed to decode public key: {}", e))?;
        keypair.append(&mut public);
        ed25519_dalek::Keypair::from_bytes(&keypair)
            .map_err(|e| format_err!("failed to generate keypair: {}", e))
            .map(|pair| {
                KeypairManager(pair)
            })
    }

    pub fn drain(self) -> ed25519_dalek::Keypair {
        self.0
    }
}
