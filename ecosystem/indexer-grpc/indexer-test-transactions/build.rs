// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use std::{env, fs, path::Path};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generate_transactions.rs");

    let mut all_transactions_code = String::new();
    let mut name_function_code = String::new();

    // Create necessary directories if missing
    create_directory_if_missing("json_transactions/imported_mainnet_txns");
    create_directory_if_missing("json_transactions/imported_testnet_txns");
    create_directory_if_missing("json_transactions/scripted_transactions");

    // Process different directories and collect name mappings
    all_transactions_code.push_str(&process_directory(
        "imported_mainnet_txns",
        "IMPORTED_MAINNET_TXNS",
        false, // Don't generate names for mainnet transactions
        &mut name_function_code,
    ));
    all_transactions_code.push_str(&process_directory(
        "imported_testnet_txns",
        "IMPORTED_TESTNET_TXNS",
        false,
        &mut name_function_code,
    ));
    all_transactions_code.push_str(&process_directory(
        "scripted_transactions",
        "SCRIPTED_TRANSACTIONS",
        true, // Generate names only for scripted transactions
        &mut name_function_code,
    ));

    if !name_function_code.is_empty() {
        all_transactions_code.push_str(&generate_get_transaction_name(&name_function_code));
    }

    fs::write(dest_path, all_transactions_code).unwrap();
}

// Helper function to process each directory and generate code for constants
fn process_directory(
    dir_name: &str,
    module_name: &str,
    generate_name_function: bool,
    name_function_code: &mut String,
) -> String {
    let mut transactions_code = String::new();
    let mut all_constants = String::new();
    let json_dir = Path::new("json_transactions").join(dir_name);

    for entry in fs::read_dir(json_dir).expect("Failed to read directory") {
        let entry = entry.expect("Failed to get directory entry");
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let file_name = path.file_stem().unwrap().to_str().unwrap();
            let const_name = format!(
                "{}_{}",
                module_name.to_uppercase(),
                file_name.to_uppercase().replace('-', "_")
            );

            let json_code = format!(
                r#"
                pub const {const_name}: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/json_transactions/{dir_name}/{file_name}.json"));
                "#,
                const_name = const_name,
                dir_name = dir_name,
                file_name = file_name,
            );
            transactions_code.push_str(&json_code);
            all_constants.push_str(&format!("{},", const_name));

            // Only generate name function for scripted transactions
            // we need to function to map txn const to name of it, to output db json file with that name
            // reason why we are not using the version is to avoid file not found.
            // scripted txn data may change when we re-generate.
            if generate_name_function {
                name_function_code.push_str(&format!(
                    "        {const_name} => Some(\"{file_name}\"),\n",
                    const_name = const_name,
                    file_name = file_name
                ));
            }
        }
    }

    if !all_constants.is_empty() {
        transactions_code.push_str(&format!(
            "pub const ALL_{}: &[&[u8]] = &[{}];\n",
            module_name.to_uppercase(),
            all_constants
        ));
    }

    transactions_code
}

// Helper function to generate the get_transaction_name function
fn generate_get_transaction_name(name_function_code: &str) -> String {
    let mut fn_code = String::new();
    fn_code.push_str(
        r#"
        pub fn get_transaction_name(const_data: &[u8]) -> Option<&'static str> {
            match const_data {
        "#,
    );

    // Add the dynamically generated match arms for scripted transactions
    fn_code.push_str(name_function_code);

    fn_code.push_str(
        r#"
                _ => None,
            }
        }
        "#,
    );

    fn_code
}

// Helper function to create directories if they are missing
fn create_directory_if_missing(dir: &str) {
    let path = Path::new(dir);
    if !path.exists() {
        fs::create_dir_all(path).expect("Failed to create directory");
    }
}
