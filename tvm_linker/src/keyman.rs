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
use ed25519_dalek::{Keypair, PublicKey, SecretKey};
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

    pub fn from_secret_file(file: &str) -> Self {
        let mut file = File::open(file.to_string()).expect(&format!("error: cannot open key file {}",file));
        let mut keys_buf = vec![];
        file.read_to_end(&mut keys_buf).unwrap();
        let pair = Keypair::from_bytes(&keys_buf).expect("error: invalid key");
        KeypairManager { pair }
    }

    #[allow(dead_code)]
    pub fn from_public_file(file: &str) -> Self {
        let mut file = File::open(file.to_string()).expect(&format!("error: cannot open public key file {}", file));
        let mut key_buf = vec![];
        file.read_to_end(&mut key_buf).unwrap();
        let pubkey = PublicKey::from_bytes(&key_buf).expect("error: invalid public key");
        KeypairManager { 
            pair : Keypair {
                secret: SecretKey::from_bytes(&[0u8; 32]).unwrap(), 
                public: pubkey,
            }
        }
    }

    pub fn store_secret(&self, file: &str) {
        let bytes = self.pair.to_bytes();
        let mut file = File::create(file.to_string()).expect(&format!("error: cannot create key file {}", file));
        file.write_all(&bytes).unwrap();
    }

    pub fn store_public(&self, file: &str) {
        let bytes = self.pair.public.to_bytes();
        let mut file = File::create(file.to_string()).expect(&format!("error: cannot create key file {}", file));
        file.write_all(&bytes).unwrap();
    }

    pub fn drain(self) -> Keypair {
        self.pair
    }
}

fn generate_keypair() -> Keypair {
    let mut csprng = OsRng{};
    Keypair::generate(&mut csprng)
}