// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{aptos_core_path, components::get_execution_hash};
use anyhow::Result;
use aptos_framework::{BuildOptions, BuiltPackage, ReleasePackage};
use aptos_temppath::TempPath;
use aptos_types::account_address::AccountAddress;
use git2::Repository;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Debug)]
pub struct FrameworkReleaseConfig {
    /// Move bytecode version the framework release would be compiled to.
    pub bytecode_version: u32,
    /// Compile the framework release at a given git commit hash.
    /// If set to None, we will use the aptos framework under current repo.
    pub git_hash: Option<String>,
}

pub fn generate_upgrade_proposals(
    config: &FrameworkReleaseConfig,
    is_testnet: bool,
    next_execution_hash: Vec<u8>,
) -> Result<Vec<(String, String)>> {
    const APTOS_GIT_PATH: &str = "https://github.com/aptos-labs/aptos-core.git";

    let mut package_path_list = [
        ("0x1", "aptos-move/framework/move-stdlib"),
        ("0x1", "aptos-move/framework/aptos-stdlib"),
        ("0x1", "aptos-move/framework/aptos-framework"),
        ("0x3", "aptos-move/framework/aptos-token"),
        ("0x4", "aptos-move/framework/aptos-token-objects"),
    ];

    let mut result: Vec<(String, String)> = vec![];

    let temp_root_path = TempPath::new();
    temp_root_path.create_as_dir()?;

    let commit_info = if let Some(revision) = &config.git_hash {
        // If a commit hash is set, clone the repo from github and checkout to desired hash to a local temp directory.
        let repository = Repository::clone(APTOS_GIT_PATH, temp_root_path.path())?;
        let (commit, _) = repository.revparse_ext(revision.as_str())?;
        let commit_info = commit
            .describe(&git2::DescribeOptions::default())?
            .format(None)?;
        repository.checkout_tree(&commit, None)?;
        commit_info
    } else {
        aptos_build_info::get_git_hash()
    };

    // For generating multi-step proposal files, we need to generate them in the reverse order since
    // we need the hash of the next script.
    // We will reverse the order back when writing the files into a directory.
    if !next_execution_hash.is_empty() {
        package_path_list.reverse();
    }

    for (publish_addr, relative_package_path) in package_path_list.iter() {
        let account = AccountAddress::from_hex_literal(publish_addr)?;
        let temp_script_path = TempPath::new();
        temp_script_path.create_as_file()?;
        let mut move_script_path = temp_script_path.path().to_path_buf();
        move_script_path.set_extension("move");

        let mut package_path = if config.git_hash.is_some() {
            temp_root_path.path().to_path_buf()
        } else {
            aptos_core_path()
        };

        package_path.push(relative_package_path);

        let script_name = package_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();

        // If this file is the first framework file being generated (if `result.is_empty()` is true),
        // its `next_execution_hash` should be the `next_execution_hash` value being passed in.
        // If the `result` vector is not empty, the current file's `next_execution_hash` should be the
        // hash of the latest framework file being generated (the hash of result.last()).
        // For example, let's say we are going to generate these files:
        // 0-move-stdlib.move	2-aptos-framework.move	4-gas-schedule.move	6-features.move
        // 1-aptos-stdlib.move	3-aptos-token.move	5-version.move		7-consensus-config.move
        // The first framework file being generated is 3-aptos-token.move. It's using the next_execution_hash being passed in (so in this case, the hash of 4-gas-schedule.move being passed in mod.rs).
        // The second framework file being generated would be 2-aptos-framework.move, and it's using the hash of 3-aptos-token.move (which would be result.last()).

        let options = BuildOptions {
            with_srcs: true,
            with_abis: false,
            with_source_maps: false,
            with_error_map: true,
            skip_fetch_latest_git_deps: false,
            bytecode_version: Some(config.bytecode_version),
            ..BuildOptions::default()
        };
        let package = BuiltPackage::build(package_path, options)?;
        let release = ReleasePackage::new(package)?;

        // If we're generating a single-step proposal on testnet
        if is_testnet && next_execution_hash.is_empty() {
            release.generate_script_proposal_testnet(
                account,
                move_script_path.clone(),
                todo!("function_name"),
            )?;
            // If we're generating a single-step proposal on mainnet
        } else if next_execution_hash.is_empty() {
            release.generate_script_proposal(
                account,
                move_script_path.clone(),
                todo!("function_name"),
            )?;
            // If we're generating a multi-step proposal
        } else {
            let next_execution_hash_bytes = if result.is_empty() {
                next_execution_hash.clone()
            } else {
                get_execution_hash(&result)
            };
            release.generate_script_proposal_multi_step(
                account,
                move_script_path.clone(),
                None, //next_execution_hash_bytes,
                todo!("function_name"),
            )?;
        };

        let mut script = format!(
            "// Framework commit hash: {}\n// Builder commit hash: {}\n",
            commit_info,
            aptos_build_info::get_git_hash()
        );

        script.push_str(&std::fs::read_to_string(move_script_path.as_path())?);

        result.push((script_name, script));
    }
    Ok(result)
}
