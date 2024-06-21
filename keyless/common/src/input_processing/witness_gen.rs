// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, bail, Result};
use std::{fs, process::Command};
use tempfile::NamedTempFile;
use tracing::info_span;

pub trait PathStr {
    fn path_str(&self) -> Result<&str>;
}

impl PathStr for NamedTempFile {
    fn path_str(&self) -> Result<&str> {
        self.path().to_str().ok_or(anyhow!("tempfile path error"))
    }
}

pub fn witness_gen(
    witness_gen_js_path: &str,
    witness_gen_wasm_path: &str,
    body: &str,
) -> Result<NamedTempFile> {
    let span = info_span!("Generating witness");
    let _enter = span.enter();
    let input_file = NamedTempFile::new()?;
    let witness_file = NamedTempFile::new()?;
    fs::write(input_file.path(), body.as_bytes())?;
    let mut cmd = get_witness_command(
        witness_gen_js_path,
        witness_gen_wasm_path,
        input_file.path_str()?,
        witness_file.path_str()?,
    );
    let output = cmd.output()?;
    // Check if the command executed successfully
    if output.status.success() {
        // if config.enable_dangerous_logging {
        //     // Convert the output bytes to a string
        //     let stdout = String::from_utf8_lossy(&output.stdout);
        //     // Print the output
        //     println!("Command output:\n{}", stdout);
        // }
        Ok(witness_file)
    } else {
        // Print the error message if the command failed
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!("Command failed:\n{}\n{}", stdout, stderr)
    }
}

fn get_witness_command(
    witness_gen_js_path: &str,
    witness_gen_wasm_path: &str,
    input_file_path: &str,
    witness_file_path: &str,
) -> Command {
    let mut c = Command::new("node");
    c.args(&[
        witness_gen_js_path.to_string(),
        witness_gen_wasm_path.to_string(),
        String::from(input_file_path),
        String::from(witness_file_path),
    ]);
    c
}
