// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// build.rs
use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generate_transactions.rs");
    let mut transactions_code = String::new();
    let json_dir = Path::new("json_transactions");
    for entry in fs::read_dir(json_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let file_name = path.file_stem().unwrap().to_str().unwrap();
            let const_name = file_name.to_uppercase().replace('-', "_");

            let json_code = format!(
                r#"
                pub const {const_name}: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/json_transactions/{file_name}.json"));
                "#,
                const_name = const_name,
                file_name = file_name,
            );
            transactions_code.push_str(&json_code);
        }
    }
    fs::write(dest_path, transactions_code).unwrap();
}
