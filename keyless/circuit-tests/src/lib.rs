// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

extern crate core;

use anyhow::ensure;
use aptos_keyless_common::input_processing::{
    circuit_input_signals::{CircuitInputSignals, Padded},
    witness_gen::witness_gen,
};
use std::{env, fs, fs::File, path::PathBuf, process::Command};
use tempfile::{tempdir, NamedTempFile, TempDir};

#[cfg(test)]
mod base64;
#[cfg(test)]
mod packing;
#[cfg(test)]
mod sha;
#[cfg(test)]
mod arrays;
#[cfg(test)]
mod rsa;

pub struct TestCircuitHandle {
    dir: TempDir,
}

impl TestCircuitHandle {
    pub fn new(file_name: &str) -> anyhow::Result<Self> {
        let dir = tempdir()?;
        let cargo_manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        let include_root_dir = cargo_manifest_dir.join("../circuit-data/templates");
        let src_circuit_path = include_root_dir.join("tests").join(file_name);
        let tmp_circuit_path = dir.path().to_owned().join("circuit.circom");
        // Rex: why is this variable never used?
        let _tmp_circuit_file = File::create(&tmp_circuit_path)?;
        fs::copy(src_circuit_path, &tmp_circuit_path)?;
        let output = Command::new("circom")
            .args([
                "-l",
                include_root_dir.to_str().unwrap(),
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
