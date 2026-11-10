// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! CLI of the configuration recorder; see the library documentation.

use anyhow::{bail, Context, Result};
use std::fs;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut output: Option<String> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" | "-o" => output = Some(args.next().context("--output needs a path")?),
            _ => bail!("usage: move-bytecode-lean-config-export [--output FILE]"),
        }
    }
    let text = serde_json::to_string_pretty(&move_bytecode_lean_config_export::document())? + "\n";
    match output {
        Some(path) => fs::write(&path, text).with_context(|| format!("writing {}", path))?,
        None => print!("{}", text),
    }
    Ok(())
}
