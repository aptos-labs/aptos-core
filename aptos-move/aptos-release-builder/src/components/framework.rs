// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::components::get_execution_hash;
use anyhow::Result;
use aptos_temppath::TempPath;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct FrameworkReleaseConfig {
    pub bytecode_version: u32,
}

pub fn generate_upgrade_proposals(
    config: &FrameworkReleaseConfig,
    is_testnet: bool,
    next_execution_hash: Vec<u8>,
) -> Result<Vec<(String, String)>> {
    let mut package_path_list = vec![
        ("0x1", "aptos-move/framework/move-stdlib"),
        ("0x1", "aptos-move/framework/aptos-stdlib"),
        ("0x1", "aptos-move/framework/aptos-framework"),
        ("0x3", "aptos-move/framework/aptos-token"),
    ];

    let mut result: Vec<(String, String)> = vec![];

    let mut root_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    root_path.pop();
    root_path.pop();

    // For generating multi-step proposal files, we need to generate them in the reverse order since
    // we need the hash of the next script.
    // We will reverse the order back when writing the files into a directory.
    if !next_execution_hash.is_empty() {
        package_path_list.reverse();
    }

    for (publish_addr, relative_package_path) in package_path_list.iter() {
        let temp_script_path = TempPath::new();
        temp_script_path.create_as_file()?;
        let mut move_script_path = temp_script_path.path().to_path_buf();
        move_script_path.set_extension("move");

        let mut package_path = root_path.clone();
        package_path.push(relative_package_path);

        let script_name = package_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        let bytecode_version = format!("{:?}", config.bytecode_version);

        let mut args = vec![
            "run",
            "--bin",
            "aptos",
            "--",
            "governance",
            "generate-upgrade-proposal",
            "--account",
            publish_addr,
            "--output",
            move_script_path.to_str().unwrap(),
            "--package-dir",
            package_path.to_str().unwrap(),
            "--bytecode-version",
            bytecode_version.as_str(),
        ];

        if is_testnet {
            args.push("--testnet");
        }

        // If this file is the first framework file being generated (if `result.is_empty()` is true),
        // its `next_execution_hash` should be the `next_execution_hash` value being passed in.
        // If the `result` vector is not empty, the current file's `next_execution_hash` should be the
        // hash of the latest framework file being generated (the hash of result.last()).
        // For example, let's say we are going to generate these files:
        // 0-move-stdlib.move	2-aptos-framework.move	4-gas-schedule.move	6-features.move
        // 1-aptos-stdlib.move	3-aptos-token.move	5-version.move		7-consensus-config.move
        // The first framework file being generated is 3-aptos-token.move. It's using the next_execution_hash being passed in (so in this case, the hash of 4-gas-schedule.move being passed in mod.rs).
        // The second framework file being generated would be 2-aptos-framework.move, and it's using the hash of 3-aptos-token.move (which would be result.last()).
        let mut _next_execution_hash_string = "".to_owned();
        if !next_execution_hash.clone().is_empty() {
            args.push("--next-execution-hash");
            // Convert from bytes to string to pass next_execution_hash to the command line
            if result.is_empty() {
                _next_execution_hash_string = hex::encode(next_execution_hash.clone());
            } else {
                _next_execution_hash_string = hex::encode(get_execution_hash(&result));
            }
            args.push(&_next_execution_hash_string);
        }

        assert!(Command::new("cargo")
            .current_dir(root_path.as_path())
            .args(args)
            .output()
            .unwrap()
            .status
            .success());

        let script = std::fs::read_to_string(move_script_path.as_path())?;

        result.push((script_name, script));
    }
    Ok(result)
}
