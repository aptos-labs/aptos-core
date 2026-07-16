// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Generate sample bytecode-v10 `.mv` artifacts for exercising the WASM API.
//!
//! Writes two files next to the given output directory (default: `samples/`):
//! - `sample_v10_module.mv` — a compiled module serialized at bytecode v10
//! - `sample_v10_script.mv` — a compiled script serialized at bytecode v10
//!
//! Run with:
//! ```bash
//! cargo run --example gen_samples            # writes to ./samples
//! cargo run --example gen_samples -- out_dir # custom output directory
//! ```

use move_binary_format::{
    file_format::{basic_test_module, basic_test_script},
    file_format_common::VERSION_10,
};
use std::{fs, path::PathBuf};

fn main() -> anyhow::Result<()> {
    let out_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("samples"));
    fs::create_dir_all(&out_dir)?;

    let mut module_bytes = Vec::new();
    basic_test_module().serialize_for_version(Some(VERSION_10), &mut module_bytes)?;
    let module_path = out_dir.join("sample_v10_module.mv");
    fs::write(&module_path, &module_bytes)?;
    println!(
        "wrote {} ({} bytes, bytecode v{})",
        module_path.display(),
        module_bytes.len(),
        VERSION_10
    );

    let mut script_bytes = Vec::new();
    basic_test_script().serialize_for_version(Some(VERSION_10), &mut script_bytes)?;
    let script_path = out_dir.join("sample_v10_script.mv");
    fs::write(&script_path, &script_bytes)?;
    println!(
        "wrote {} ({} bytes, bytecode v{})",
        script_path.display(),
        script_bytes.len(),
        VERSION_10
    );

    Ok(())
}
