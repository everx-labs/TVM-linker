/*
 * Copyright 2018-2024 EverX Labs Ltd.
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
use base64::encode;

use std::fs::File;
use std::io::{Read, Write};

use std::time::SystemTime;
use ever_block::*;

use ever_block::{
    read_boc, Cell, SliceData, BuilderData, Result,
};

const XMODEM: crc::Crc<u16> = crc::Crc::<u16>::new(&crc::CRC_16_XMODEM);

pub fn save_to_file(state: StateInit, name: Option<&str>, wc: i8, silent: bool) -> Result<String> {
    let buffer = state.write_to_bytes()?;

    let mut print_filename = false;
    let address = state.hash().unwrap();
    let file_name = if let Some(name) = name {
        name.to_string()
    } else {
        print_filename = true;
        format!("{:x}.tvc", address)
    };

    let mut file = File::create(&file_name)?;
    file.write_all(&buffer)?;

    if print_filename {
        if silent {
            println!("{{\n  \"output_path\":\"{}\"\n}}", &file_name);
        } else {
            println!("Saved contract to file {}", &file_name);
            println!("testnet:");
            println!("Non-bounceable address (for init): {}", &calc_userfriendly_address(wc, address.as_slice(), false, true));
            println!("Bounceable address (for later access): {}", &calc_userfriendly_address(wc, address.as_slice(), true, true));
            println!("mainnet:");
            println!("Non-bounceable address (for init): {}", &calc_userfriendly_address(wc, address.as_slice(), false, false));
            println!("Bounceable address (for later access): {}", &calc_userfriendly_address(wc, address.as_slice(), true, false));
        }
    }
    Ok(file_name)
}

fn calc_userfriendly_address(wc: i8, addr: &[u8], bounce: bool, testnet: bool) -> String {
    let mut bytes: Vec<u8> = vec![];
    bytes.push(if bounce { 0x11 } else { 0x51 } + if testnet { 0x80 } else { 0 });
    bytes.push(wc as u8);
    bytes.extend_from_slice(addr);
    let crc = XMODEM.checksum(&bytes);
    bytes.extend_from_slice(&crc.to_be_bytes());
    encode(&bytes)
}

pub fn load_from_file(contract_file: &str) -> Result<StateInit> {
    let mut cell = read_boc(std::fs::read(contract_file)?)?.roots.remove(0);
    // try appending a dummy library cell if there is no such cell in the tvc file
    if cell.references_count() == 2 {
        let mut adjusted_cell = BuilderData::from_cell(&cell)?;
        adjusted_cell.checked_append_reference(Cell::default())?;
        cell = adjusted_cell.into_cell()?;
    }
    StateInit::construct_from_cell(cell)
}

pub fn load_stateinit(file_name: &str) -> Result<(SliceData, Vec<u8>)> {
    let mut orig_bytes = Vec::new();
    let mut f = File::open(file_name)?;
    f.read_to_end(&mut orig_bytes)?;

    let mut root = read_boc(orig_bytes.clone())?.roots.remove(0);
    if root.references_count() == 2 { // append empty library cell
        let mut adjusted_cell = BuilderData::from_cell(&root)?;
        adjusted_cell.checked_append_reference(Cell::default())?;
        root = adjusted_cell.into_cell()?;
    }
    Ok((SliceData::load_cell(root)?, orig_bytes))
}

pub fn get_now() -> u32 {
    SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bouncable_address() {
        let addr = hex::decode("fcb91a3a3816d0f7b8c2c76108b8a9bc5a6b7a55bd79f8ab101c52db29232260").unwrap();
        let addr = calc_userfriendly_address(-1, &addr, true, true);
        assert_eq!(addr, "kf/8uRo6OBbQ97jCx2EIuKm8Wmt6Vb15+KsQHFLbKSMiYIny");
    }
}
