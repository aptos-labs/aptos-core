// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_temppath::TempPath;
use std::process::Command;

pub fn generate_upgrade_proposals(is_testnet: bool) -> Result<Vec<(String, String)>> {
    let package_path_list = vec![
        ("0x1", "aptos-move/framework/move-stdlib"),
        ("0x1", "aptos-move/framework/aptos-stdlib"),
        ("0x1", "aptos-move/framework/aptos-framework"),
        ("0x3", "aptos-move/framework/aptos-token"),
    ];

    let mut result = vec![];

    let mut root_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    root_path.pop();
    root_path.pop();

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

        assert!(Command::new("cargo")
            .current_dir(root_path.as_path())
            .args(&vec![
                "run",
                "--bin",
                "aptos",
                "--",
                "governance",
                "generate-upgrade-proposal",
                if is_testnet { "--testnet" } else { "" },
                "--account",
                publish_addr,
                "--package-dir",
                package_path.to_str().unwrap(),
                "--output",
                move_script_path.to_str().unwrap(),
            ])
            .output()
            .unwrap()
            .status
            .success());

        let script = std::fs::read_to_string(move_script_path.as_path())?;

        result.push((script_name, script));
    }
    Ok(result)
}
