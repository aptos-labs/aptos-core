// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::components::feature_flags::Features;
use anyhow::{anyhow, Result};
use aptos_types::on_chain_config::{GasScheduleV2, OnChainConsensusConfig, Version};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

pub mod consensus_config;
pub mod feature_flags;
pub mod framework;
pub mod gas;
pub mod version;

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct ReleaseConfig {
    pub testnet: bool,
    pub framework_release: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas_schedule: Option<GasScheduleV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<Version>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_flags: Option<Features>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consensus_config: Option<OnChainConsensusConfig>,
}

impl ReleaseConfig {
    pub fn generate_release_proposal_scripts(&self, base_path: &Path) -> Result<()> {
        let mut result = vec![];

        // First create framework releases
        if self.framework_release {
            result.append(&mut framework::generate_upgrade_proposals(self.testnet)?);
        }

        if let Some(gas_schedule) = &self.gas_schedule {
            result.append(&mut gas::generate_gas_upgrade_proposal(
                gas_schedule,
                self.testnet,
            )?);
        }

        if let Some(version) = &self.version {
            result.append(&mut version::generate_version_upgrade_proposal(
                version,
                self.testnet,
            )?);
        }

        if let Some(feature_flags) = &self.feature_flags {
            result.append(&mut feature_flags::generate_feature_upgrade_proposal(
                feature_flags,
                self.testnet,
            )?);
        }

        if let Some(consensus_config) = &self.consensus_config {
            result.append(&mut consensus_config::generate_consensus_upgrade_proposal(
                consensus_config,
                self.testnet,
            )?);
        }

        for (idx, (script_name, script)) in result.into_iter().enumerate() {
            let mut script_path = base_path.to_path_buf();
            let proposal_name = format!("{}-{}", idx, script_name);
            script_path.push(&proposal_name);
            script_path.set_extension("move");

            std::fs::write(script_path.as_path(), script.as_bytes())
                .map_err(|err| anyhow!("Failed to write to file: {:?}", err))?;
        }
        Ok(())
    }

    pub fn load_config<P: AsRef<Path>>(path: P) -> Result<Self> {
        // Open the file and read it into a string
        let config_path_string = path.as_ref().to_str().unwrap().to_string();
        let mut file = File::open(&path).map_err(|error| {
            anyhow!(
                "Failed to open config file: {:?}. Error: {:?}",
                config_path_string,
                error
            )
        })?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|error| {
            anyhow!(
                "Failed to read the config file into a string: {:?}. Error: {:?}",
                config_path_string,
                error
            )
        })?;

        // Parse the file string
        Self::parse(&contents)
    }

    pub fn save_config<P: AsRef<Path>>(&self, output_file: P) -> Result<()> {
        let contents =
            serde_yaml::to_vec(&self).map_err(|e| anyhow!("failed to generate config: {:?}", e))?;
        let mut file = File::create(output_file.as_ref())
            .map_err(|e| anyhow!("failed to create file: {:?}", e))?;
        file.write_all(&contents)
            .map_err(|e| anyhow!("failed to write file: {:?}", e))?;
        Ok(())
    }

    pub fn parse(serialized: &str) -> Result<Self> {
        serde_yaml::from_str(serialized).map_err(|e| anyhow!("Failed to parse the config: {:?}", e))
    }
}

impl Default for ReleaseConfig {
    fn default() -> Self {
        ReleaseConfig {
            testnet: true,
            framework_release: true,
            gas_schedule: Some(aptos_gas::gen::current_gas_schedule()),
            version: None,
            feature_flags: None,
            consensus_config: Some(OnChainConsensusConfig::default()),
        }
    }
}
