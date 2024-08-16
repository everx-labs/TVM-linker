/*
 * Copyright 2018-2024 EverX Labs Ltd.
 *
 * Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
 * this file except in compliance with the License.
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific EVERX DEV software governing permissions and
 * limitations under the License.
 */
use anyhow::format_err;
use ever_block::{ed25519_create_private_key, Ed25519PrivateKey, Ed25519PublicKey, Result};
use serde::Deserialize;

pub struct Keypair {
    pub private: Ed25519PrivateKey,
    pub public: Ed25519PublicKey,
}

impl Keypair {
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
        let private = hex::decode(keys.secret)
            .map_err(|e| format_err!("failed to decode private key: {}", e))?;
        let public = hex::decode(keys.public)
            .map_err(|e| format_err!("failed to decode public key: {}", e))?;

        let public_bytes = public
            .try_into()
            .map_err(|v: Vec<u8>| format_err!("failed to get public bytes, bad len {}", v.len()))?;
        Ok(Self {
            private: ed25519_create_private_key(&private)?,
            public: Ed25519PublicKey::from_bytes(&public_bytes)?,
        })
    }
}
