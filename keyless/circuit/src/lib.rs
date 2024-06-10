// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

extern crate core;

use anyhow::ensure;
use aptos_keyless_common::input_processing::{
    circuit_input_signals::{CircuitInputSignals, Padded},
    witness_gen::witness_gen,
};
use std::{env, fs, fs::File, io::Write, path::PathBuf, process::Command};
use tempfile::{tempdir, NamedTempFile, TempDir};

#[cfg(test)]
mod arrays;
#[cfg(test)]
mod base64;
#[cfg(test)]
mod bigint;
#[cfg(test)]
mod hash_to_field;
#[cfg(test)]
mod jwt_field_parsing;
#[cfg(test)]
mod misc;
#[cfg(test)]
mod packing;
#[cfg(test)]
mod rsa;
#[cfg(test)]
mod sha;

pub struct TestCircuitHandle {
    dir: TempDir,
}

impl TestCircuitHandle {
    /// Compile the circuit in the given file using BN254 as the underlying curve.
    pub fn new(file_name: &str) -> anyhow::Result<Self> {
        let cargo_manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let include_root_dir = cargo_manifest_dir.join("./templates");
        let src_circuit_path = include_root_dir.join("tests").join(file_name);
        let content = fs::read_to_string(src_circuit_path)?;
        Self::new_from_str(content.as_str())
    }

    pub fn new_from_str(circuit_src: &str) -> anyhow::Result<Self> {
        let dir = tempdir()?;
        let cargo_manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let include_root_dir = cargo_manifest_dir.join("./templates");
        let tmp_circuit_path = dir.path().to_owned().join("circuit.circom");
        let mut tmp_circuit_file = File::create(&tmp_circuit_path)?;
        let global_node_modules_path =
            String::from_utf8(Command::new("npm").args(["root", "-g"]).output()?.stdout).unwrap();
        tmp_circuit_file.write_all(circuit_src.as_bytes())?;
        let output = Command::new("circom")
            .args([
                "-l",
                include_root_dir.to_str().unwrap(),
                "-l",
                global_node_modules_path.trim(),
                tmp_circuit_path.to_str().unwrap(),
                "--c",
                "--wasm",
                "-o",
                dir.path().to_str().unwrap(),
            ])
            .output()?;
        println!("{}", String::from_utf8_lossy(&output.stdout));
        println!("{}", String::from_utf8_lossy(&output.stderr));
        ensure!(output.status.success());
        Ok(Self { dir })
    }

    pub fn gen_witness(
        &self,
        input_signals: CircuitInputSignals<Padded>,
    ) -> anyhow::Result<NamedTempFile> {
        let formatted_input_str = serde_json::to_string(&input_signals.to_json_value())?;
        witness_gen(
            self.witness_gen_js_path().to_str().unwrap(),
            self.witness_gen_wasm_path().to_str().unwrap(),
            &formatted_input_str,
        )
    }

    fn witness_gen_js_path(&self) -> PathBuf {
        self.dir
            .path()
            .to_owned()
            .join("circuit_js/generate_witness.js")
    }

    fn witness_gen_wasm_path(&self) -> PathBuf {
        self.dir.path().to_owned().join("circuit_js/circuit.wasm")
    }
}

// pub fn run_circuit_test(circuit_name: &str, circuit_input_signals: CircuitInputSignals<Padded>) {
//     // compute circuit input signals (input.json)
//     let formatted_input_str = serde_json::to_string(&circuit_input_signals.to_json_value()).unwrap();
//     // run witness generation phase for `circuit_name`
//     let compile_circuit();
//     witness_gen(witness_gen_js_path, witness_gen_wasm_path, &formatted_input_str).unwrap();
// }
