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
use ever_abi::{
    json_abi::{decode_function_response, encode_function_call},
    Contract,
};

use crate::keyman::Keypair;
use ever_block::{BuilderData, Result, SliceData};

pub fn build_abi_body(
    abi_file: &str,
    method: &str,
    params: &str,
    header: Option<&str>,
    keypair: Option<Keypair>,
    internal: bool,
    address: Option<String>,
) -> Result<BuilderData> {
    encode_function_call(
        &load_abi_json_string(abi_file)?,
        method,
        header,
        params,
        internal,
        keypair.map(|p| p.private).as_ref(),
        address.as_deref(),
    )
}

pub fn load_abi_json_string(abi_file: &str) -> Result<String> {
    std::fs::read_to_string(abi_file)
        .map_err(|e| format_err!("unable to read ABI file {}: {}", abi_file, e))
}

pub fn load_abi_contract(abi_json: &str) -> Result<Contract> {
    Contract::load(abi_json.as_bytes())
        .map_err(|e| format_err!("cannot parse contract abi: {:?}", e))
}

pub fn decode_body(
    abi_file: &str,
    method: &str,
    body: SliceData,
    internal: bool,
) -> Result<String> {
    decode_function_response(
        &load_abi_json_string(abi_file)?,
        method,
        body,
        internal,
        false,
    )
}
