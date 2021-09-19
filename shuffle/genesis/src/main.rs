// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use diem_config::config::NodeConfig;
use diem_types::on_chain_config::VMPublishingOption;
use std::path::Path;
use structopt::StructOpt;

/// Directory where Move source code lives
const MOVE_CODE_DIR: &str = "../move/";
/// Directory where Move module bytecodes live
const MOVE_BYTECODE_DIR: &str = "../move/storage/0x00000000000000000000000000000001/modules";

#[derive(StructOpt)]
#[structopt(
    name = "custom-node",
    about = "Builds a WriteSet transaction to install the custom modules and starts a node",
    rename_all = "kebab-case"
)]
pub struct CustomFramework {
    /// Directory where the node config will be generated. Must not already exist
    #[structopt(long = "node-config-dir")]
    node_config_dir: String,
    #[structopt(long = "open-publishing")]
    open_publishing: bool,
}
/// Generate a node config under `args.node_config_dir`
fn main() -> Result<()> {
    let args = CustomFramework::from_args();
    shuffle_custom_node::build_move_sources(Path::new(MOVE_CODE_DIR))?;
    let publishing_option = if args.open_publishing {
        VMPublishingOption::open() // everyone can publish modules and execute custom scripts
    } else {
        VMPublishingOption::custom_scripts() // everyone can execute custom scripts
    };
    let validator_config = shuffle_custom_node::generate_validator_config(
        Path::new(&args.node_config_dir),
        Path::new(MOVE_BYTECODE_DIR),
        publishing_option,
    )?;
    let node_config = NodeConfig::load(validator_config.config_path())?;
    println!("Running a Diem node with custom modules ...");
    diem_node::start(&node_config, None);
    Ok(())
}
