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
//extern crate ton_abi;
//extern crate base64;
#[macro_use]
extern crate clap;
extern crate crc16;
//extern crate ed25519_dalek;
//#[macro_use]
//extern crate lazy_static;
//extern crate rand;
//extern crate regex;
//extern crate serde_json;
//extern crate sha2;
//extern crate ton_block;
//extern crate ton_types;
//#[macro_use]
//extern crate ton_vm;
extern crate ton_client_rs;

mod config;
mod deploy;
mod genaddr;
mod helpers;
mod call;

use call::call_contract;
use clap::ArgMatches;
use config::Config;
use deploy::deploy_contract;
use genaddr::generate_address;

fn main() -> Result<(), i32> {
    println!(
        "tonlabs-cli {}\nCOMMIT_ID: {}\nBUILD_DATE: {}\nCOMMIT_DATE: {}\nGIT_BRANCH: {}",
        env!("CARGO_PKG_VERSION"),
        env!("BUILD_GIT_COMMIT"),
        env!("BUILD_TIME") ,
        env!("BUILD_GIT_DATE"),
        env!("BUILD_GIT_BRANCH")
    );
    main_internal().map_err(|err_str| {
        println!("Error: {}", err_str);
        1
    })
}

fn main_internal() -> Result <(), String> {
    let build_info = match option_env!("BUILD_INFO") {
        Some(s) => s,
        None => "",
    };

    let matches = clap_app! (tonlabs_cli =>        
        (version: &*format!("0.1 ({})", build_info))
        (author: "TONLabs")
        (about: "TONLabs console tool for TON")
        (@subcommand genaddr =>
            (@setting AllowNegativeNumbers)
            (about: "Calculate smart contract address in different formats.")
            (version: "0.1")
            (author: "TONLabs")
            (@arg TVC: +required +takes_value "Compiled smart contract (tvc file)")
            (@arg WC: --wc +takes_value "Workchain id used to generate user-friendly addresses (default -1).")
            (@arg GENKEY: --genkey +takes_value conflicts_with[SETKEY] "Generates new keypair for the contract and saves it to the file")
            (@arg SETKEY: --setkey +takes_value conflicts_with[GENKEY] "Loads existing keypair from the file")
        )
        (@subcommand deploy =>
            (@setting AllowNegativeNumbers)
            (about: "Deploy smart contract to blockchain.")
            (version: "0.1")
            (author: "TONLabs")
            (@arg TVC: +required +takes_value "Compiled smart contract (tvc file)")
            (@arg ABI: +required +takes_value "Json file with contract ABI.")
            (@arg PARAMS: +required +takes_value "Constructor arguments.")
            (@arg SIGN: --sign +takes_value "Keypair used to sign 'constructor message'.")
            (@arg WC: --wc +takes_value "Workchain id used to print contract address, -1 by default.")            
        )
        (@subcommand send =>
            (about: "Sends external message to contract with encoded function call.")
            (version: "0.1")
            (author: "TONLabs")
            (@arg ADDRESS: +required +takes_value "Contract address.")
            (@arg BODY: --body +takes_value "Raw body as hex string.")
            (@arg MSG: --message +takes_value "File with message boc.")
            (@arg ABI_JSON: --abi +takes_value conflicts_with[BODY] "Supplies json file with contract ABI")
            (@arg ABI_METHOD: --method +takes_value conflicts_with[BODY] "Supplies the name of the calling contract method")
            (@arg ABI_PARAMS: --params +takes_value conflicts_with[BODY] "Supplies ABI arguments for the contract method")
            (@arg SIGN: --sign +takes_value "Keypair used to sign message.")
        )
        (@subcommand run =>
            (about: "Runs contract's get-method.")
            (version: "0.1")
            (author: "TONLabs")
            (@arg ADDRESS: +required +takes_value "Contract address.")
            (@arg ABI_JSON: --abi +takes_value conflicts_with[BODY] "Supplies json file with contract ABI")
            (@arg ABI_METHOD: --method +takes_value conflicts_with[BODY] "Supplies the name of the calling contract method")
            (@arg ABI_PARAMS: --params +takes_value conflicts_with[BODY] "Supplies ABI arguments for the contract method")
        )
        (@subcommand config =>
            (about: "Writes parameters to config file that can be used later in subcommands.")
            (version: "0.1")
            (author: "TONLabs")
            (@arg URL: --url +takes_value "Supplies url to connect.")
            (@arg ABI: --abi +takes_value conflicts_with[DATA] "File with contract ABI.")
            (@arg METHOD: --method +takes_value "The name of the calling contract method.")
            (@arg PARAMS: --params +takes_value "Arguments for the contract method.")
            (@arg KEYS: --keys +takes_value "File with keypair.")
            (@arg ADDR: --addr +takes_value "Contract address.")
        )
        (@setting SubcommandRequired)
    ).get_matches();

    let conf = Config::new();

    if let Some(send_matches) = matches.subcommand_matches("send") {
        return send_command(send_matches, conf);
    }
    if let Some(run_matches) = matches.subcommand_matches("run") {
        return run_command(run_matches, conf);
    }
    if let Some(deploy_matches) = matches.subcommand_matches("deploy") {        
        return deploy_command(deploy_matches, conf);
    } 
    if let Some(config_matches) = matches.subcommand_matches("config") {
        return config_command(config_matches, conf);
    }
    if let Some(genaddr_matches) = matches.subcommand_matches("genaddr") {
        return genaddr_command(genaddr_matches, conf);
    }
    Err("invalid arguments".to_string())
}

fn send_command(matches: &ArgMatches, config: Config) -> Result<(), String> {
    let addr = matches.value_of("ADDRESS").unwrap();
    let abi = matches.value_of("ABI").unwrap();
    let method = matches.value_of("METHOD").unwrap();
    let params = matches.value_of("PARAMS").unwrap();
    let keys = matches.value_of("SIGN");
    call_contract(config, addr, abi, method, params, keys, false)
}

fn run_command(matches: &ArgMatches, config: Config) -> Result<(), String> {
    let addr = matches.value_of("ADDRESS").unwrap();
    let abi = matches.value_of("ABI").unwrap();
    let method = matches.value_of("METHOD").unwrap();
    let params = matches.value_of("PARAMS").unwrap();
    call_contract(config, addr, abi, method, params, None, false)
}

fn deploy_command(matches: &ArgMatches, config: Config) -> Result<(), String> {
    let tvc = matches.value_of("TVC").unwrap();
    let abi = matches.value_of("ABI").unwrap();
    let params = matches.value_of("PARAMS").unwrap();
    let keys = matches.value_of("SIGN").ok_or("keypair file is required (--sign option)")?;
    deploy_contract(config, tvc, abi, params, keys)
}

fn config_command(matches: &ArgMatches, config: Config) -> Result<(), String> {
    Ok(())
}

fn genaddr_command(matches: &ArgMatches, config: Config) -> Result<(), String> {
    let tvc = matches.value_of("TVC").unwrap();
    let wc = matches.value_of("WC");
    let keys = matches.value_of("GENKEY").or(matches.value_of("SETKEY"));
    let new_keys = matches.is_present("GENKEY");
    generate_address(config, tvc, wc, keys, new_keys)
}
