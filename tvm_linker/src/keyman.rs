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
use ed25519_dalek::{Keypair};
use rand::rngs::OsRng;
use std::fs::File;
use std::io::{Read, Write};

pub struct KeypairManager {
    pair: Keypair,
}

impl KeypairManager {
    pub fn new() -> Self {
        KeypairManager {
            pair: generate_keypair()
        }
    }

    pub fn from_secret_file(file: &str) -> Option<Self> {
        read_key(file).ok().map_or(None, |buf| {
            Keypair::from_bytes(&buf).ok().map_or(None, |pair| {
                Some(KeypairManager { pair })
            })
        })
    }

    pub fn store_secret(&self, file: &str) -> Result<(), String> {
        self.store_key(file, true)
    }

    pub fn store_public(&self, file: &str) -> Result<(), String> {
        self.store_key(file, false)
    }

    fn store_key(&self, file: &str, is_secret: bool) -> Result<(), String> {
        let bytes = match is_secret {
            true => self.pair.to_bytes().to_vec(),
            false => self.pair.public.to_bytes().to_vec()
        };
        let mut file = File::create(file.to_string())
            .map_err(|e| format!("Failed to create key file {}: {}", file, e))?;
        file.write_all(&bytes).map_err(|e| format!("Failed to save key: {}", e))?;
        Ok(())
    }

    pub fn drain(self) -> Keypair {
        self.pair
    }
}

fn generate_keypair() -> Keypair {
    let mut csprng = OsRng{};
    Keypair::generate(&mut csprng)
}


fn read_key(file_path: &str) -> Result<Vec<u8>, ()> {
    let mut file = File::open(file_path.to_string())
        .map_err(|e| println!("Failed to open the key file {}: {}", file_path, e))?;
    let mut keys_buf = vec![];
    file.read_to_end(&mut keys_buf)
        .map_err(|e| println!("Failed to open the key file {}: {}", file_path, e))?;
    Ok(keys_buf)
}