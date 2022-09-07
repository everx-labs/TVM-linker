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
        let filename = format!("tests/disasm/{:03}.b64", n);
        let raw = std::fs::read_to_string(filename.clone()).unwrap();
        round_trip_test(&raw, false);
    }
    for n in 200..331 {
        let filename = format!("tests/disasm/{:03}.b64", n);
        let raw = std::fs::read_to_string(filename.clone()).unwrap();
        round_trip_test(&raw, true);
    }
}
