// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use std::{fs, path::Path};

pub const IMPORTED_MAINNET_TXNS: &str = "imported_mainnet_txns";
pub const IMPORTED_TESTNET_TXNS: &str = "imported_testnet_txns";
pub const IMPORTED_DEVNET_TXNS: &str = "imported_devnet_txns";
pub const SCRIPTED_TRANSACTIONS_TXNS: &str = "scripted_transactions";

#[derive(Default)]
pub struct TransactionCodeBuilder {
    // Holds the generated Rust code for transaction constants
    transactions_code: String,
    // Holds the match arms for the name generation function for scripted txns (optional)
    name_function_code: String,
}

impl TransactionCodeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /**
     * Adds a directory of JSON files to the transaction code builder.
     *
     * @param src_dir: The source directory containing the JSON files
     * @param dir_name: The name of the directory to be created in the `json_transactions` directory
     * @param module_name: The name of the module to be used in the constant names
     * @param generate_name_function: Whether to generate a transaction name lookup function
     */
    pub fn add_directory(
        mut self,
        json_dir: &Path,
        module_name: &str,
        generate_name_function: bool,
    ) -> Self {
        let mut all_constants = String::new();
        // Iterates over all files in the directory
        if !json_dir.exists() {
            let _ = fs::create_dir_all(json_dir);
        }

        for entry in fs::read_dir(json_dir).expect("Failed to read directory") {
            let entry = entry.expect("Failed to get directory entry");
            let path = entry.path();

            // Checks if the file has a `.json` extension
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                let file_name = path.file_stem().unwrap().to_str().unwrap();
                let const_name = format!(
                    "{}_{}",
                    module_name.to_uppercase(),
                    file_name.to_uppercase().replace('-', "_")
                );

                // Generates a constant for the JSON file and appends it to the `transactions_code` string
                self.transactions_code.push_str(&format!(
                    r#"
                    pub const {const_name}: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/json_transactions/{dir_name}/{file_name}.json"));
                    "#,
                    const_name = const_name,
                    dir_name = module_name,
                    file_name = file_name,
                ));

                // Adds the constant to the list of all constants
                all_constants.push_str(&format!("{},", const_name));

                // If name function generation is requested, adds the corresponding match arm
                if generate_name_function {
                    self.name_function_code.push_str(&format!(
                        "        {const_name} => Some(\"{file_name}\"),\n",
                        const_name = const_name,
                        file_name = file_name
                    ));
                }
            }
        }

        // If any constants were created, generate an array holding all of them
        if !all_constants.is_empty() {
            self.transactions_code.push_str(&format!(
                "pub const ALL_{}: &[&[u8]] = &[{}];\n",
                module_name.to_uppercase(),
                all_constants
            ));
        }

        self
    }

    pub fn add_license_in_comments(mut self) -> Self {
        self.transactions_code.push_str(
            r#"
                    // Copyright (c) Velor Foundation
                    // SPDX-License-Identifier: Apache-2.0
                    #![allow(dead_code)]
                    #![allow(unused_variables)]
                "#,
        );
        self
    }

    // Adds the transaction name lookup function if any name match arms were created
    pub fn add_transaction_name_function(mut self) -> Self {
        if !self.name_function_code.is_empty() {
            self.transactions_code.push_str(
                r#"
                pub fn get_transaction_name(const_data: &[u8]) -> Option<&'static str> {
                    match const_data {
                "#,
            );

            self.transactions_code.push_str(&self.name_function_code);

            self.transactions_code.push_str(
                r#"
                    _ => None,
                }
            }
            "#,
            );
        }
        self
    }

    pub fn build(self) -> String {
        self.transactions_code
    }
}
