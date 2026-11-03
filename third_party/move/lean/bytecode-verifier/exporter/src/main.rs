// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! CLI of the exporter; see the library documentation.

use anyhow::{bail, Context, Result};
use move_bytecode_lean_export::{
    document, instr, instruction_catalog, load_inputs, write_output, FORMAT,
};
use serde_json::{json, Value};

const USAGE: &str = "usage: move-bytecode-lean-export [--output FILE] INPUT...\n\
                     \x20      move-bytecode-lean-export [--output FILE] --instruction-catalog\n\
                     Each INPUT is a `.mv` module or script, or a `.masm` assembly file.";

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut output: Option<String> = None;
    let mut inputs: Vec<String> = vec![];
    let mut catalog = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" | "-o" => output = Some(args.next().context("--output needs a path")?),
            "--instruction-catalog" => catalog = true,
            "--help" | "-h" => {
                println!("{}", USAGE);
                return Ok(());
            },
            _ => inputs.push(arg),
        }
    }
    if catalog {
        if !inputs.is_empty() {
            bail!(USAGE);
        }
        let instructions: Vec<Value> = instruction_catalog().iter().map(instr).collect();
        return write_output(
            output,
            json!({ "format": FORMAT, "instructions": instructions }),
        );
    }
    if inputs.is_empty() {
        bail!(USAGE);
    }
    let (modules, scripts) = load_inputs(&inputs)?;
    write_output(output, document(&modules, &scripts))
}
