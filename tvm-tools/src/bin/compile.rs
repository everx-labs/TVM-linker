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

#![cfg_attr(feature = "ci_run", deny(warnings))]

extern crate ton_labs_assembler;
extern crate ton_types;
extern crate ton_vm as tvm;
extern crate clap;

use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::path::{Path, PathBuf};
use ton_types::cells_serialization::serialize_tree_of_cells;
use ton_labs_assembler::compile_code;
use clap::{Arg, App};

fn save(data: Vec<u8>, destination: &Path) {
    let destination_path_display = destination.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&destination) {
        Err(why) => panic!(
            "couldn't create {}: {}",
            destination_path_display,
            why
        ),
        Ok(file) => file,
    };

    match file.write_all(data.as_slice()) {
        Err(why) => panic!(
            "couldn't write to {}: {}",
            destination_path_display,
            why
        ),
        Ok(_) => println!("successfully saved to {}", destination_path_display),
    }
}

fn compile_contract(
    source_path: &Path,
    destination_cells_path: &Path,
) {
    let source_path_display = source_path.display();
    let mut codefile = match File::open(source_path) {
        Ok(f) => f,
        Err(e) => panic!(
            "Cannot find source file {}, error {}",
            source_path_display, e
        ),
    };

    println!("\nCompiling...");

    let mut source = String::new();
    match codefile.read_to_string(&mut source) {
        Ok(_) => (),
        Err(e) => panic!(
            "Cannot read source file {}, error {}",
            source_path_display, e
        ),
    }

    let code = match compile_code(&source) {
        Ok(code) => code,
        Err(e) => panic!("Compilation error in {}: {}", source_path_display, e),
    };

    println!("{} ", code);

    let bytecode_formatter = |vec_bytes: &mut Vec<u8>| {
        vec_bytes.pop();
        let final_bytecode = "{".to_string();
        let mut bytecode = vec_bytes.iter().fold(
            String::new(),
            |acc, &byte| acc + &format!("0x{:02X},", byte));
        bytecode.pop();
        final_bytecode + &bytecode + "};"
    };

    println!("C/C++ bytecode {} ", bytecode_formatter(&mut code.storage().to_vec()));

    let mut bag_of_cells = vec![];
    serialize_tree_of_cells(&code.into_cell(), &mut bag_of_cells)
        .unwrap_or_else(|err| panic!("BOC serialize error: {}", err));
    save(bag_of_cells, destination_cells_path);
}

fn file_path<T>(source_code_path: &Path, new_extension: T) -> PathBuf
where
    T: Into<String>,
{
    let mut destination_filename = source_code_path.file_stem().unwrap().to_os_string();
    destination_filename.push(new_extension.into());
    source_code_path.with_file_name(destination_filename)
}

fn main() {
    println!("Compile {}\nCOMMIT_ID: {}\nBUILD_DATE: {}\nCOMMIT_DATE: {}\nGIT_BRANCH: {}",
            env!("CARGO_PKG_VERSION"),
            env!("BUILD_GIT_COMMIT"),
            env!("BUILD_TIME") ,
            env!("BUILD_GIT_DATE"),
            env!("BUILD_GIT_BRANCH"));
    let args = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::with_name("FILES")
            .help("path to smart contract files")
            .required(true)
            .multiple(true)
            .index(1))
        .get_matches();

    args.values_of("FILES").map(|paths| {
        for path in paths.map(|e| Path::new(e)) {
            compile_contract(
                path,
                Path::new(&file_path(&path, ".cells")),
            );
        }
    });
}
