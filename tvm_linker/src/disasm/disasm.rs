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

use std::collections::HashSet;
use std::str::FromStr;
use ton_block::Serializable;
use clap::ArgMatches;
use ton_types::cells_serialization::deserialize_cells_tree;
use ton_types::{Cell, HashmapE, HashmapType, SliceData, UInt256};
use std::io::Cursor;

use super::types::Shape;
use super::loader::{load, print_code};

pub fn disasm_command(m: &ArgMatches) -> core::result::Result<(), String> {
    if let Some(m) = m.subcommand_matches("dump") {
        return disasm_dump_command(m);
    } else if let Some(m) = m.subcommand_matches("graphviz") {
        return disasm_graphviz_command(m);
    } else if let Some(m) = m.subcommand_matches("text") {
        return disasm_text_command(m);
    }
    Err("unknown command".to_owned())
}

fn disasm_graphviz_command(m: &ArgMatches) -> core::result::Result<(), String> {
    let filename = m.value_of("TVC");
    let tvc = filename.map(|f| std::fs::read(f))
        .transpose()
        .map_err(|e| format!(" failed to read tvc file: {}", e))?
        .unwrap();
    let mut csor = Cursor::new(tvc);
    let mut roots = deserialize_cells_tree(&mut csor).unwrap();
    let root = roots.remove(0).reference(0).unwrap();
    match m.value_of("METHOD") {
        Some(string) => {
            if string == "int" {
                graphviz(&root.reference(1).unwrap())
            } else if string == "ext" {
                graphviz(&root.reference(2).unwrap())
            } else if string == "ticktock" {
                graphviz(&root.reference(3).unwrap())
            } else {
                let method_id = u32::from_str(string).map_err(|e| -> String { e.to_string() })?;
                let dict_cell = root.reference(0).unwrap().reference(0).unwrap();
                let dict = HashmapE::with_hashmap(32, Some(dict_cell));
                if dict.len().is_err() {
                    return Err("empty internal methods dictionary".to_string())
                }
                let key = method_id.serialize().unwrap().into();
                let data = dict.get(key).map_err(|e| -> String { e.to_string() })?
                    .ok_or(format!("internal method {} not found", method_id))?;
                let cell = data.into_cell();
                graphviz(&cell)
            }
        },
        None => graphviz(&root)
    }
    Ok(())
}

fn data_pretty_printed(cell: &Cell) -> String {
    let mut string = String::new();
    let mut hex = cell.to_hex_string(true);
    if hex.len() > 0 {
        while hex.len() > 32 {
            let tail = hex.split_off(32);
            string += format!("<tr><td align=\"left\">{}</td></tr>", hex).as_str();
            hex = tail;
        }
        string += format!("<tr><td align=\"left\">{}</td></tr>", hex).as_str();
    } else {
        string += "<tr><td align=\"left\">8_</td></tr>";
    }
    string
}

fn tree_walk_graphviz(cell: &Cell, visited: &mut HashSet<UInt256>) {
    visited.insert(cell.repr_hash());
    let cell_hash = cell.repr_hash().to_hex_string();
    let cell_id = &cell_hash.as_str()[..8];
    println!("  \"{}\" [label=<<table border=\"0\"><tr><td align=\"left\"><b>{}</b></td></tr>{}</table>>];",
        cell_id, cell_id, data_pretty_printed(cell));
    if cell.references_count() > 0 {
        for i in 0..cell.references_count() {
            let child = cell.reference(i).unwrap();
            let child_hash = child.repr_hash().to_hex_string();
            let child_id = &child_hash.as_str()[..8];
            println!("  \"{}\" -> \"{}\" [ taillabel=\"{}\"];", cell_id, child_id, i.to_string());
        }
    }
    for i in 0..cell.references_count() {
        let child = cell.reference(i).unwrap();
        if !visited.contains(&child.repr_hash()) {
            tree_walk_graphviz(&child, visited);
        }
    }
}

fn graphviz(cell: &Cell) {
    println!("digraph code {{");
    println!("  node [shape=box, fontname=\"DejaVu Sans Mono\"]");
    let mut visited = HashSet::new();
    tree_walk_graphviz(&cell, &mut visited);
    println!("}}");
}

fn disasm_dump_command(m: &ArgMatches) -> core::result::Result<(), String> {
    let filename = m.value_of("TVC");
    let tvc = filename.map(|f| std::fs::read(f))
        .transpose()
        .map_err(|e| format!(" failed to read tvc file: {}", e))?
        .unwrap();
    let mut csor = Cursor::new(tvc);
    let roots = deserialize_cells_tree(&mut csor).map_err(|e| e.to_string())?;
    if roots.len() == 0 {
        println!("empty");
    } else {
        println!("{} {} in total", roots.len(), if roots.len() < 2 { "root" } else { "roots" });
        for i in 0..roots.len() {
            let root = roots.get(i).unwrap();
            println!("root {}:", i);
            print_tree_of_cells(root);
        }
    }
    Ok(())
}

pub(super) fn print_tree_of_cells(toc: &Cell) {
    fn print_tree_of_cells(cell: &Cell, prefix: String, last: bool) {
        let indent = if last { "└ " } else { "├ " };
        let mut hex = cell.to_hex_string(true);
        if hex.len() > 0 {
            let mut first = true;
            let indent_next = if !last { "│ " } else { "  " };
            while hex.len() > 64 {
                let tail = hex.split_off(64);
                println!("{}{}{}…", prefix, if first { indent } else { indent_next }, hex);
                hex = tail;
                first = false;
            }
            println!("{}{}{}", prefix, if first { indent } else { indent_next }, hex);
        } else {
            println!("{}{}{}", prefix, indent, "8_");
        }

        let prefix_child = if last { "  " } else { "│ " };
        let prefix = prefix + prefix_child;
        if cell.references_count() > 0 {
            let last_child = cell.references_count() - 1;
            for i in 0..cell.references_count() {
                let child = cell.reference(i).unwrap();
                print_tree_of_cells(&child, prefix.to_string(), i == last_child);
            }
        }
    }
    print_tree_of_cells(&toc, "".to_string(), true);
}

fn print_code_dict(cell: &Cell, key_size: usize) {
    let dict = HashmapE::with_hashmap(key_size, Some(cell.clone()));
    if dict.len().is_err() {
        println!("failed to recognize dictionary");
        return
    }
    for (key, slice) in dict.iter().map(|r| r.unwrap()) {
        let cell = key.into_cell().unwrap();
        let id = SliceData::from(cell).get_next_int(key_size).unwrap();
        println!("");
        println!(";; function id 0x{:x}", id);
        print!("{}", disasm(&mut slice.clone()));
    }
}

fn disasm_text_command(m: &ArgMatches) -> core::result::Result<(), String> {
    let shape_deprecated = Shape::literal("ff00f4a42022c00192f4a0e18aed535830f4a1")
        .branch(Shape::var("dict-public"))
        .branch(Shape::literal("f4a420f4a1")
            .branch(Shape::var("dict-c3")));

    let shape_current = Shape::literal("8aed5320e30320c0ffe30220c0fee302f20b")
        .branch(Shape::var("dict-c3"))
        .branch(Shape::var("internal"))
        .branch(Shape::var("external"))
        .branch(Shape::var("ticktock"));

    let shape_current_mycode = Shape::literal("8adb35")
        .branch(Shape::literal("20f861ed1ed9"))
        .branch(Shape::literal("8aed5320e30320c0ffe30220c0fee302f20b")
            .branch(Shape::var("dict-c3"))
            .branch(Shape::var("internal"))
            .branch(Shape::var("external"))
            .branch(Shape::var("ticktock")));

    let shape_fun_c = Shape::literal("ff00f4a413f4bcf2c80b")
        .branch(Shape::var("dict-c3")
            .branch(Shape::any())); // just to mark any() as used, can be omitted

    let filename = m.value_of("TVC");
    let tvc = filename.map(|f| std::fs::read(f))
        .transpose()
        .map_err(|e| format!(" failed to read tvc file: {}", e))?
        .unwrap();
    let mut csor = Cursor::new(tvc);
    let mut roots = deserialize_cells_tree(&mut csor).map_err(|e| e.to_string())?;
    let code = roots.remove(0).reference(0).unwrap();

    if let Ok(assigned) = shape_deprecated.captures(&code) {
        println!(";; solidity deprecated selector detected");
        println!(";; public methods dictionary");
        print_code_dict(&assigned["dict-public"], 32);
        println!(";; internal functions dictionary");
        print_code_dict(&assigned["dict-c3"], 32);
    } else if let Ok(assigned) = shape_current.captures(&code)
            .or_else(|_| shape_current_mycode.captures(&code)) {
        println!(";; solidity selector detected");
        println!(";; internal functions dictionary");
        print_code_dict(&assigned["dict-c3"], 32);
        println!(";; internal transaction entry point");
        println!("{}", disasm(&mut SliceData::from(&assigned["internal"])));
        println!(";; external transaction entry point");
        println!("{}", disasm(&mut SliceData::from(&assigned["external"])));
        println!(";; ticktock transaction entry point");
        println!("{}", disasm(&mut SliceData::from(&assigned["ticktock"])));
    } else if let Ok(assigned) = shape_fun_c.captures(&code) {
        println!(";; fun-c selector detected");
        println!(";; internal functions dictionary");
        print_code_dict(&assigned["dict-c3"], 19);
    } else {
        return Err("failed to recognize selector".to_string())
    }

    Ok(())
}

pub(super) fn disasm(slice: &mut SliceData) -> String {
    print_code(&load(slice).unwrap(), "")
}
