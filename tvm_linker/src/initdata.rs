use ed25519_dalek::PublicKey;
use program::save_to_file;
use std::fs::OpenOptions;
use ton_sdk;

pub fn set_initial_data(tvc: &str, pubkey: Option<[u8; 32]>, data: &str, abi: &str) -> Result<(), String> {
    let mut state_init = OpenOptions::new().read(true).open(tvc)
        .map_err(|e| format!("unable to open contract file: {}", e))?;
    let abi = std::fs::read_to_string(abi)
        .map_err(|e| format!("unable to read ABI file: {}", e))?;

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