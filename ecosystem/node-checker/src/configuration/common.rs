// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::types::NodeConfiguration;
use crate::evaluators::build_evaluators;
use anyhow::{anyhow, Context, Result};
use std::{
    convert::{TryFrom, TryInto},
    fs::File,
    path::PathBuf,
};

enum FileType {
    Yaml(PathBuf),
    Json(PathBuf),
}

impl TryFrom<PathBuf> for FileType {
    type Error = anyhow::Error;

    fn try_from(path: PathBuf) -> Result<Self> {
        let extension = path
            .extension()
            .ok_or_else(|| anyhow!("Config file must have an extension"))?;
        match extension
            .to_str()
            .ok_or_else(|| anyhow!("Invalid extension"))?
        {
            "yaml" | "yml" => Ok(FileType::Yaml(path)),
            "json" => Ok(FileType::Json(path)),
            wildcard => Err(anyhow!(
                "File extension must be yaml, yml, or json, not {}",
                wildcard
            )),
        }
    }
}

impl TryInto<NodeConfiguration> for FileType {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<NodeConfiguration> {
        match self {
            FileType::Yaml(path) => {
                let file = File::open(&path)?;
                let node_configuration: NodeConfiguration = serde_yaml::from_reader(file)
                    .with_context(|| format!("{} was not valid YAML", path.display()))?;
                Ok(node_configuration)
            }
            FileType::Json(path) => {
                let file = File::open(&path)?;
                let node_configuration: NodeConfiguration = serde_json::from_reader(file)
                    .with_context(|| format!("{} was not valid JSON", path.display()))?;
                Ok(node_configuration)
            }
        }
    }
}

pub fn read_configuration_from_file(path: PathBuf) -> Result<NodeConfiguration> {
    let file_type = FileType::try_from(path)?;
    file_type.try_into()
}

pub fn validate_configuration(node_configuration: &NodeConfiguration) -> Result<()> {
    build_evaluators(
        &node_configuration.evaluators,
        &node_configuration.evaluator_args,
    )
    .context("Failed to build evaluators")?;
    Ok(())
}
