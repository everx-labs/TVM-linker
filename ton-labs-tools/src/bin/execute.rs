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

#![cfg_attr(feature = "ci_run", deny(warnings))]

extern crate ton_vm as tvm;
extern crate ton_labs_tools;
extern crate clap;
#[allow(unused_imports)]
#[macro_use]
extern crate log;
use clap::{Arg, App,SubCommand};
use std::error::Error;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::path::{Path,PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::thread;
use tvm::executor::Engine;
use ton_labs_tools::{
    Contract,
    ContractsRepository,
    FileBasedContractsRepository
};
use tvm::stack::{
    IntegerData, SliceData, Stack, StackItem, SaveList
};
use tvm::types::AccountId;

fn as_contract_id(filename: &OsStr) -> AccountId {
    AccountId::from_str(
        filename.to_str().expect("Invalid file name format: expecting utf-8")
    )
    .expect("Invalid file name format")
}

fn load_raw_contract(path: &Path) -> SliceData {
    let path_display = path.display();
    let mut contract_file = match File::open(path) {
        Ok(f) => f,
        Err(e) => panic!("{}, error {}", path_display, e.description()),
    };
    let mut contract: Vec<u8> = Vec::new();
    match contract_file.read_to_end(&mut contract) {
        Ok(_) => (),
        Err(e) => panic!(
            "Cannot read contract {}, error {}",
            path_display,
            e.description()
        ),
    }
    println!("Contract: {:X?}", contract);
    SliceData::new(contract)
}

fn execute_contract(contract_path: &Path, initial_stack_state: Stack) {

    let contracts_source_dir = contract_path.parent().unwrap_or(Path::new("./"));
    let contract_filename = contract_path.file_stem().expect("Contract name must be specified");
    let raw = contract_path.extension() != Some(OsStr::new("contract"));
    let (id, contracts);
    let (code, data) = if raw {
        id = AccountId::from([0u8;32]);
        contracts = None;
        (load_raw_contract(contract_path), SliceData::new_empty())
    } else {
        id = as_contract_id(&contract_filename);
        contracts = Some(FileBasedContractsRepository::new(move |id| {
            PathBuf::from(contracts_source_dir).join(format!("{:x}.contract", id))
        }));
        let contract = contracts.as_ref().and_then(|ref contracts| contracts.find(&id))
            .expect("Contract was not found or corrupt");
        (contract.code().clone(), contract.persistent_data().clone())
    };

    let mut ctrls = SaveList::new();
    ctrls.put(4, &mut StackItem::Cell(data.into_cell())).unwrap();
    let mut engine = Engine::new().setup(code.clone(), Some(ctrls), Some(initial_stack_state), None);
    let exit_code = match engine.execute() {
        Err(exc) => {
            println!("Unhandled exception: {}", exc); 
            exc.number
        },
        Ok(code) => code as usize,
    };
    println!("TVM terminated with exit code {}", exit_code);
    if exit_code != 0 && exit_code != 1 {
        return;
    }
    if let Some(contracts) = contracts {
        let data = engine.get_committed_state().get_root().as_cell().expect("c4 register must contain a Cell").into();
        contracts.store(&id, &Contract::create(code.clone(), data));
    }
    // Output is checked by sol2tvm in this format:
    // --- Post-execution stack state ---------
    // ----------------------------------------
    println!("{}", engine.dump_stack("Post-execution stack state", false));

}

fn parse_slice(v: &str) -> Result<SliceData, String> {
    hex::decode(v)
        .map(|v| SliceData::new(v))
        .map_err(|e| format!("Invalid slice data: {}", e))
}

fn parse_item(v: String) -> Result<StackItem, String> {
    // Currently we support two types of params: <uint> and <slice>.
    // Slice accepts a hexadecimal string. Please note that completion tag should be included,
    // i.e. to pass one byte 0xFF you have to use "slice:0xFF80" parameter.
    const UINT_PREFIX: &str = "uint:";
    const SLICE_PREFIX: &str = "slice:";
    if v.starts_with(UINT_PREFIX) {
        let (_, value) = v.split_at(UINT_PREFIX.len());
        let mut radix = 10;
        let mut str_slice = &value[..];
        if value.starts_with('x') {
            radix = 16;
            str_slice = &value[1..];
        }
        return IntegerData::from_str_radix(str_slice, radix)
            .map(|e| StackItem::Integer(Arc::new(e)))
            .map_err(|e| format!("{}", e));
    } else if v.starts_with(SLICE_PREFIX) {
        let (_, value) = v.split_at(SLICE_PREFIX.len());
        return parse_slice(value)
            .map(|s| StackItem::Slice(s))
            .map_err(|e| format!("{}", e));
    }
    Err(String::from("The value did not contain the required type. Usage: uint:<value> | slice:<hex-string-with-completion-tag>"))
}

fn has_type(v: String) -> Result<(), String> {
    parse_item(v).map(|_| ())
}

fn main() {
    println!("Execute {}\nCOMMIT_ID: {}\nBUILD_DATE: {}\nCOMMIT_DATE: {}\nGIT_BRANCH: {}",
            env!("CARGO_PKG_VERSION"),
            env!("BUILD_GIT_COMMIT"),
            env!("BUILD_TIME") ,
            env!("BUILD_GIT_DATE"),
            env!("BUILD_GIT_BRANCH"));
    let handle = thread::Builder::new().stack_size(5000000).spawn(all_in_thread);
    handle.unwrap().join().unwrap();
}

fn all_in_thread() {
    let args = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("FILE")
            .help("path to smart contract file")
            .required(true)
            .index(1))
        .subcommand(SubCommand::with_name("--params")
            .help("Contract parameters to put on stack")
            .arg(Arg::with_name("stack-items")
                .value_name("stack-item")
                .validator(has_type)
                .number_of_values(1)
                .multiple(true))
        )
        .get_matches();
    let mut initial_stack_state = Stack::new();
    if let Some(params) = args.subcommand_matches("--params") {
        params.values_of("stack-items")
            .map(|items| {
                for item in items.collect::<Vec<_>>() {
                    let stack_item = parse_item(item.to_string())
                        .expect("Passed validation");
                    initial_stack_state.push(stack_item);
                }
            });
    }
    args.value_of("FILE").map(move |path| execute_contract(Path::new(path), initial_stack_state));
}
