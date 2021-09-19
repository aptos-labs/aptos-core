// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::new::Config;
use anyhow::Result;
use diem_config::config::NodeConfig;
use std::{fs, path::Path};

pub fn handle(project_path: &Path) -> Result<()> {
    // TODO: Generate prefunded accounts
    let node_config = NodeConfig::load(project_path.join("nodeconfig/0/node.yaml").as_path())?;
    let config = read_config(project_path)?;
    println!(
        "running shuffle node configured for {} in {}",
        &config.blockchain,
        project_path.display()
    );
    diem_node::start(&node_config, None);
    Ok(())
}

fn read_config(project_path: &Path) -> Result<Config> {
    let config_string =
        fs::read_to_string(project_path.join("Shuffle").with_extension("toml")).unwrap();
    let read_config: Config = toml::from_str(config_string.as_str())?;
    Ok(read_config)
}
