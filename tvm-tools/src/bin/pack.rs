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

#![cfg_attr(feature = "ci_run", deny(warnings))]

extern crate ton_types;
extern crate ton_vm as tvm;
extern crate tvm_tools;
extern crate clap;

use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::str::FromStr;
use std::path::{
    Path,
    PathBuf,
};
use tvm::stack::SliceData;

use ton_types::cells_serialization::deserialize_tree_of_cells;
use tvm::types::AccountId;
use tvm_tools::{
    Contract,
    ContractsRepository,
    FileBasedContractsRepository
};
use clap::{
    Arg,
    App,
};

fn load_cells(path: &Path) -> SliceData {
    let mut buffer: Vec<u8> = vec![];
    File::open(path).expect(&format!("File not found: {}", path.display()))
        .read_to_end(&mut buffer)
        .unwrap_or_else(|e| panic!(format!("File: {}. I/O error {}", path.display(), e)));
    deserialize_tree_of_cells(&mut std::io::Cursor::new(buffer))
        .unwrap_or_else(|err| panic!("BOC load error: {}", err)).into()
}

fn as_contract_id(filename: &OsStr) -> AccountId {
    AccountId::from_str(
        filename.to_str().expect("Invalid file name format: expecting utf-8")
    )
    .expect("Invalid file name format")
}

fn pack_contract(dest_path: PathBuf, cc_path: PathBuf, root_data_path: Option<PathBuf>) {
    let dest_contracts_dir = dest_path.as_path().parent().unwrap_or(Path::new("./"));
    let contracts = FileBasedContractsRepository::new(move |id| {
        PathBuf::from(dest_contracts_dir).join(format!("{:x}.contract", id))
    });
    let contract_id = as_contract_id(
        dest_path.file_stem()
            .expect("Destination account (contract) must be specified")
    );
    let code = load_cells(cc_path.as_path());
    let data = root_data_path
        .map(|p| load_cells(p.as_path()))
        .unwrap_or_else(|| SliceData::new_empty());

    contracts.store(&contract_id, &Contract::create(
        code,
        data
    ));
}

fn main() {
    println!("Pack {}\nCOMMIT_ID: {}\nBUILD_DATE: {}\nCOMMIT_DATE: {}\nGIT_BRANCH: {}",
            env!("CARGO_PKG_VERSION"),
            env!("BUILD_GIT_COMMIT"),
            env!("BUILD_TIME") ,
            env!("BUILD_GIT_DATE"),
            env!("BUILD_GIT_BRANCH"));
    let args = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("account")
            .help("destination smart contract")
            .takes_value(true)
            .required(true)
        )
        .arg(Arg::with_name("code-continuation")
            .long("cc")
            .help("path to smart contract code entry point (cc)")
            .takes_value(true)
            .required(true)
        )
        .arg(Arg::with_name("persistent-data")
            .help("Sets the initial data to use")
            .long("c4")
            .long("root-data")
            .takes_value(true)
            .required(false)
        )
        .get_matches();
    let dest_contract_path = args.value_of("account").unwrap();
    let cc_path = args.value_of("code-continuation").unwrap();
    let root_data = args.value_of("persistent-data"); 
    pack_contract(
        Path::new(dest_contract_path).to_path_buf(),
        Path::new(cc_path).to_path_buf(), 
        root_data.map(|path| Path::new(path).to_path_buf())
    );
}
