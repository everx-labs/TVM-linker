/*
 * Copyright 2018-2021 TON DEV SOLUTIONS LTD.
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

use ton_types::SliceData;
use super::commands::{disasm, print_tree_of_cells};

fn round_trip_test(raw0: &str, check_bin: bool) {
    let bin0 = base64::decode(raw0).unwrap();
    let toc0 = ton_types::deserialize_tree_of_cells(&mut std::io::Cursor::new(bin0)).unwrap();
    let asm0 = disasm(&mut SliceData::from(toc0.clone()));
    let toc1 = ton_labs_assembler::compile_code_to_cell(&asm0.clone()).unwrap();
    let asm1 = disasm(&mut SliceData::from(toc1.clone()));
    if asm0 != asm1 {
        println!(">>>");
        print!("{}", asm0);
        println!("<<<");
        print!("{}", asm1);
        assert!(false);
    }

    if check_bin {
        let bin1 = ton_types::serialize_toc(&toc1).unwrap();
        let raw1 = base64::encode(&bin1);
        if raw0 != raw1 {
            println!("{}", asm0);
            print_tree_of_cells(&toc0);
            print_tree_of_cells(&toc1);
            assert!(false);
        }
    }
}

#[test]
fn round_trip() {
    for n in 0..130 {
        let skip_check_bin = [105, 109, 113, 116, 117, 118, 119, 129].contains(&n);
        // In general, a difference in binaries is completely legit since there are many ways
        // to encode the same code: even a linear sequence of insns can be laid out into
        // a sequence of cells in many different ways, thanks to implicit jumps.
        // TODO However, sometimes the difference may be an indicator of some CQ issue
        // in the assembler.
        let filename = format!("tests/disasm/{:03}.b64", n);
        let raw = std::fs::read_to_string(filename).unwrap();
        round_trip_test(&raw, !skip_check_bin);
    }
    for n in 200..331 {
        let filename = format!("tests/disasm/{:03}.b64", n);
        let raw = std::fs::read_to_string(filename).unwrap();
        round_trip_test(&raw, true);
    }
}

#[test]
fn round_trip_tonix() {
    let paths = std::fs::read_dir("tests/disasm/tonix-ea2f96c/").unwrap();
    let mut tonix_base64_files = vec!();
    for entry in paths {
        let path = entry.unwrap().path();
        tonix_base64_files.push(path.to_str().unwrap().to_owned());
    }
    tonix_base64_files.sort();
    for filename in tonix_base64_files {
        println!("{}", filename);
        let raw = std::fs::read_to_string(filename).unwrap();
        round_trip_test(&raw, true);
    }
}
