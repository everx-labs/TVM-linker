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

use rayon::prelude::*;
use similar::{ChangeTag, TextDiff};

fn cut_asm_hashes(asm: String) -> String {
    let mut out = String::new();
    for line in asm.lines() {
        if let Some((before, _)) = line.split_once(" ;; #") {
            out += &format!("{}\n", before);
        } else {
            out += &format!("{}\n", line);
        }
    }
    out
}

fn round_trip_test(filename: &str, check_bin: bool) {
    let raw0 = &std::fs::read_to_string(filename).unwrap();
    let bin0 = base64::decode(raw0).unwrap();
    let toc0 = ton_types::deserialize_tree_of_cells(&mut std::io::Cursor::new(bin0)).unwrap();
    let mut asm0 = disasm(&mut SliceData::load_cell(toc0.clone()).unwrap());
    let toc1 = ton_labs_assembler::compile_code_to_cell(&asm0.clone()).unwrap();
    let mut asm1 = disasm(&mut SliceData::load_cell(toc1.clone()).unwrap());

    if !check_bin {
        asm0 = cut_asm_hashes(asm0);
        asm1 = cut_asm_hashes(asm1);
    }

    let diff = TextDiff::from_lines(&asm0, &asm1);
    let mut differ = false;
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Delete => {
                print!("-{}", change);
                differ = true;
            }
            ChangeTag::Insert => {
                print!("+{}", change);
                differ = true;
            }
            _ => ()
        }
    }
    assert!(!differ, "roundtrip difference was detected for {}", filename);

    if check_bin {
        let bin1 = ton_types::serialize_toc(&toc1).unwrap();
        let raw1 = base64::encode(&bin1);
        if raw0 != &raw1 {
            println!("{}", asm0);
            print_tree_of_cells(&toc0);
            print_tree_of_cells(&toc1);
            assert!(false);
        }
    }
}

#[test]
fn round_trip() {
    let mut indices = (0..130).collect::<Vec<i32>>();
    indices.append(&mut (200..331).collect());
    for n in indices {
        round_trip_test(&format!("tests/disasm/{:03}.b64", n), true);
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
    tonix_base64_files.par_iter().for_each(|filename| {
        round_trip_test(&filename, true);
    })
}

fn check(code: &str, text: &str) {
    let mut slice = SliceData::from_string(code).unwrap();
    let text_disasm = disasm(&mut slice);
    assert_eq!(text, &text_disasm);
}

#[test]
fn disasm_fragment() {
    check("70", "PUSHINT 0\n");
    check("88", "PUSHREF {\n  ;; missing cell\n}\n");
    check("8b04", "PUSHSLICE x4_\n");
    check("8c0800000000", "PUSHSLICE x000000004_\n");
    check("8c40", "PUSHSLICE x4_ ;; missing 1 ref\n");
    check("8c80", "PUSHSLICE x4_ ;; missing 2 refs\n");
    check("8e80", "PUSHCONT {\n} ;; missing 1 ref\n");
    check("8e81", "PUSHCONT {\n} ;; missing 8 bits and 1 ref\n");
    check("920000", "PUSHCONT {\n  NOP\n  NOP\n}\n");
    check("e300", "IFREF {\n  ;; missing cell\n}\n");
    check("e30f", "IFREFELSEREF {\n  ;; missing cell\n}{\n  ;; missing cell\n}\n");
    check("f4a420", "DICTPUSHCONST 32 ;; missing dict ref\n");
}
