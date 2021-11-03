// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use diem_config::config::NodeConfig;
use diem_genesis_tool::validator_builder::ValidatorConfig;
use diem_types::on_chain_config::VMPublishingOption;
use move_command_line_common::files::{extension_equals, find_filenames};
use std::{
    fs,
    io::{self, Write},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process::Command,
};

pub mod release;
pub mod utils;

pub const MOVE_EXTENSION: &str = "move";
const MOVE_MODULES_DIR: &str = "../move/src/modules";

/// The output path for transaction script ABIs.
const COMPILED_SCRIPTS_ABI_DIR: &str = "compiled/script_abis";
/// The path for Diem Framework transaction script ABIs.
const DF_COMPILED_SCRIPTS_ABI_DIR: &str =
    "../../diem-move/diem-framework/DPN/releases/artifacts/current/script_abis";
/// The output path for generated transaction builders
const TRANSACTION_BUILDERS_GENERATED_SOURCE_PATH: &str = "../transaction-builder/src/framework.rs";

pub fn generate_validator_config(
    node_config_dir: &Path,
    genesis_modules: Vec<Vec<u8>>,
    publishing_option: VMPublishingOption,
) -> Result<ValidatorConfig> {
    assert!(
        !node_config_dir.exists(),
        "We need to create node config dir {:?}, but it already exists",
        node_config_dir
    );
    fs::create_dir(node_config_dir)?;
    println!("Creating genesis with {} modules", genesis_modules.len());
    let template = NodeConfig::default_for_validator();
    std::fs::DirBuilder::new()
        .recursive(true)
        .create(&node_config_dir)
        .unwrap();
    let node_config_dir = node_config_dir.canonicalize().unwrap();
    let builder = diem_genesis_tool::validator_builder::ValidatorBuilder::new(
        &node_config_dir,
        genesis_modules,
    )
    .num_validators(NonZeroUsize::new(1).unwrap()) // start with just one validator
    .template(template)
    .randomize_first_validator_ports(false)
    .publishing_option(publishing_option);
    let (root_keys, _genesis, _genesis_waypoint, mut validators) =
        builder.build(rand::rngs::OsRng).unwrap();

    let diem_root_key_path = node_config_dir.join("mint.key");
    let serialized_keys = bcs::to_bytes(&root_keys.root_key).unwrap();
    let mut key_file = std::fs::File::create(&diem_root_key_path).unwrap();
    key_file.write_all(&serialized_keys).unwrap();

    Ok(validators.pop().unwrap())
}

pub fn build_move_sources(move_code_dir: &Path) -> Result<()> {
    // Build the Move code to ensure we get the latest changes in script builders + the genesis WriteSet
    utils::time_it("Building Move code", || {
        let output = Command::new("move")
            .args(&["sandbox", "publish", "--mode=bare"])
            .current_dir(move_code_dir)
            .output()
            .expect("Failure building Move code");
        if !output.status.success() || !output.stdout.is_empty() || !output.stderr.is_empty() {
            io::stdout().write_all(&output.stdout).unwrap();
            panic!("Automatically building Move code failed. Need to manually resolve the issue using the CLI");
        }
    });

    // Generate script ABIs
    utils::time_it("Generating script ABIs", || {
        release::generate_script_abis(Path::new(COMPILED_SCRIPTS_ABI_DIR))
    });

    // Generate script builders in Rust
    utils::time_it("Generating Rust script builders", || {
        release::generate_script_builder(
            Path::new(TRANSACTION_BUILDERS_GENERATED_SOURCE_PATH),
            &[
                Path::new(COMPILED_SCRIPTS_ABI_DIR),
                Path::new(DF_COMPILED_SCRIPTS_ABI_DIR),
            ],
        );
    });
    Ok(())
}

fn custom_move_modules_full_path() -> String {
    format!("{}/{}", env!("CARGO_MANIFEST_DIR"), MOVE_MODULES_DIR)
}

fn move_files() -> Vec<String> {
    let path = path_in_crate(MOVE_MODULES_DIR);
    find_filenames(&[path], |p| extension_equals(p, MOVE_EXTENSION)).unwrap()
}

fn path_in_crate<S>(relative: S) -> PathBuf
where
    S: Into<String>,
{
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push(relative.into());
    path
}
