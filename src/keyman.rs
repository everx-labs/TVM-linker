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
use rand::rngs::OsRng;
use std::fs::File;
use std::io::{Read, Write};
use ton_types::{Status, Result};

pub struct KeypairManager(ed25519_dalek::Keypair);

impl KeypairManager {
    pub fn new() -> Self {
        let mut rng = OsRng::default();
        let pair = ed25519_dalek::Keypair::generate(&mut rng);
        KeypairManager(pair)
    }

    pub fn from_secret_file(file: &str) -> Option<Self> {
        read_key(file).ok().and_then(|buf| {
            ed25519_dalek::Keypair::from_bytes(&buf).ok().map(|pair| {
                KeypairManager(pair)
            })
        })
    }

    pub fn store_secret(&self, file: &str) -> Status {
        self.store_key(file, true)
    }

    pub fn store_public(&self, file: &str) -> Status {
        self.store_key(file, false)
    }

    fn store_key(&self, file: &str, is_secret: bool) -> Status {
        let bytes = match is_secret {
            true => self.0.to_bytes().to_vec(),
            false => self.0.public.to_bytes().to_vec()
        };
        let mut file = File::create(file)?;
        file.write_all(&bytes)?;
        Ok(())
    }

    pub fn drain(self) -> ed25519_dalek::Keypair {
        self.0
    }
}

fn read_key(file_path: &str) -> Result<Vec<u8>> {
    let mut file = File::open(file_path)?;
    let mut keys_buf = vec![];
    file.read_to_end(&mut keys_buf)?;
    Ok(keys_buf)
}
