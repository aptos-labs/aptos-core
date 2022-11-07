// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::components::feature_flags::Features;
use anyhow::{anyhow, Result};
use aptos_types::on_chain_config::{GasScheduleV2, Version};
use std::path::Path;

pub mod feature_flags;
pub mod framework;
pub mod gas;
pub mod version;

pub struct ReleaseConfig {
    pub gas_schedule: Option<GasScheduleV2>,
    pub version: Option<Version>,
    pub feature_flags: Option<Features>,
}

impl ReleaseConfig {
    pub fn generate_release_proposal_scripts(
        &self,
        base_path: &Path,
        is_testnet: bool,
    ) -> Result<()> {
        let mut result = vec![];

        // First create framework releases
        result.append(&mut framework::generate_upgrade_proposals(is_testnet)?);

        if let Some(gas_schedule) = &self.gas_schedule {
            result.append(&mut gas::generate_gas_upgrade_proposal(
                gas_schedule,
                is_testnet,
            )?);
        }

        if let Some(version) = &self.version {
            result.append(&mut version::generate_version_upgrade_proposal(
                version, is_testnet,
            )?);
        }

        if let Some(feature_flags) = &self.feature_flags {
            result.append(&mut feature_flags::generate_feature_upgrade_proposal(
                feature_flags,
                is_testnet,
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
}

impl Default for ReleaseConfig {
    fn default() -> Self {
        ReleaseConfig {
            gas_schedule: Some(aptos_gas::gen::current_gas_schedule()),
            version: None,
            feature_flags: None,
        }
    }
}
