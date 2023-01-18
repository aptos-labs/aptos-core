// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use self::framework::FrameworkReleaseConfig;
use crate::components::feature_flags::Features;
use anyhow::{anyhow, Result};
use aptos::governance::GenerateExecutionHash;
use aptos_rest_client::Client;
use aptos_temppath::TempPath;
use aptos_types::{
    account_config::CORE_CODE_ADDRESS,
    on_chain_config::{GasScheduleV2, OnChainConfig, OnChainConsensusConfig, Version},
};
use futures::executor::block_on;
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};
use url::Url;

pub mod consensus_config;
pub mod feature_flags;
pub mod framework;
pub mod gas;
pub mod transaction_fee;
pub mod version;

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct ReleaseConfig {
    pub testnet: bool,
    pub remote_endpoint: Option<Url>,
    pub framework_release: Option<FrameworkReleaseConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gas_schedule: Option<GasScheduleV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<Version>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub feature_flags: Option<Features>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consensus_config: Option<OnChainConsensusConfig>,
    #[serde(default)]
    pub is_multi_step: bool,
}

// Compare the current on chain config with the value recorded on chain. Return false if there's a difference.
fn fetch_and_equals<T: OnChainConfig + PartialEq>(
    client: &Option<Client>,
    expected: &T,
) -> Result<bool> {
    match client {
        Some(client) => {
            let config = T::deserialize_into_config(
                block_on(async {
                    client
                        .get_account_resource_bytes(
                            CORE_CODE_ADDRESS,
                            format!(
                                "{}::{}::{}",
                                T::ADDRESS,
                                T::MODULE_IDENTIFIER,
                                T::TYPE_IDENTIFIER
                            )
                            .as_str(),
                        )
                        .await
                })?
                .inner(),
            )?;

            Ok(&config == expected)
        },
        None => Ok(false),
    }
}

impl ReleaseConfig {
    pub fn generate_release_proposal_scripts(&self, base_path: &Path) -> Result<()> {
        let mut result: Vec<(String, String)> = vec![];
        let mut release_generation_functions: Vec<
            &dyn Fn(&Self, &Option<Client>, &mut Vec<(String, String)>) -> Result<()>,
        > = vec![
            &Self::generate_framework_release,
            &Self::generate_gas_schedule,
            &Self::generate_version_file,
            &Self::generate_feature_flag_file,
            &Self::generate_consensus_file,
        ];
        let client = self
            .remote_endpoint
            .as_ref()
            .map(|url| Client::new(url.clone()));

        // If we are generating multi-step proposal files, we generate the files in reverse order,
        // since we need to pass in the hash of the next file to the previous file.
        if self.is_multi_step {
            release_generation_functions.reverse();
        }

        for f in &release_generation_functions {
            (f)(self, &client, &mut result)?;
        }

        // Here we are reversing the results back, so the result would be in order.
        if self.is_multi_step {
            result.reverse();
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

    fn generate_framework_release(
        &self,
        _client: &Option<Client>,
        result: &mut Vec<(String, String)>,
    ) -> Result<()> {
        if let Some(framework_release) = &self.framework_release {
            result.append(
                &mut framework::generate_upgrade_proposals(
                    framework_release,
                    self.testnet,
                    if self.is_multi_step {
                        get_execution_hash(result)
                    } else {
                        "".to_owned().into_bytes()
                    },
                )
                .unwrap(),
            );
        }
        Ok(())
    }

    fn generate_gas_schedule(
        &self,
        client: &Option<Client>,
        result: &mut Vec<(String, String)>,
    ) -> Result<()> {
        if let Some(gas_schedule) = &self.gas_schedule {
            if !fetch_and_equals::<GasScheduleV2>(client, gas_schedule)? {
                result.append(&mut gas::generate_gas_upgrade_proposal(
                    gas_schedule,
                    self.testnet,
                    if self.is_multi_step {
                        get_execution_hash(result)
                    } else {
                        "".to_owned().into_bytes()
                    },
                )?);
            }
        }
        Ok(())
    }

    fn generate_version_file(
        &self,
        client: &Option<Client>,
        result: &mut Vec<(String, String)>,
    ) -> Result<()> {
        if let Some(version) = &self.version {
            if !fetch_and_equals::<Version>(client, version)? {
                result.append(&mut version::generate_version_upgrade_proposal(
                    version,
                    self.testnet,
                    if self.is_multi_step {
                        get_execution_hash(result)
                    } else {
                        "".to_owned().into_bytes()
                    },
                )?);
            }
        }
        Ok(())
    }

    fn generate_feature_flag_file(
        &self,
        client: &Option<Client>,
        result: &mut Vec<(String, String)>,
    ) -> Result<()> {
        if let Some(feature_flags) = &self.feature_flags {
            let mut needs_update = true;
            if let Some(client) = client {
                let features = block_on(async {
                    client
                        .get_account_resource_bcs::<aptos_types::on_chain_config::Features>(
                            CORE_CODE_ADDRESS,
                            "0x1::features::Features",
                        )
                        .await
                })?;
                // Only update the feature flags section when there's a divergence between the local configs and on chain configs.
                needs_update = feature_flags.has_modified(features.inner());
            }
            if needs_update {
                result.append(&mut feature_flags::generate_feature_upgrade_proposal(
                    feature_flags,
                    self.testnet,
                    if self.is_multi_step {
                        get_execution_hash(result)
                    } else {
                        "".to_owned().into_bytes()
                    },
                )?);
            }
        }
        Ok(())
    }

    fn generate_consensus_file(
        &self,
        client: &Option<Client>,
        result: &mut Vec<(String, String)>,
    ) -> Result<()> {
        if let Some(consensus_config) = &self.consensus_config {
            if !fetch_and_equals(client, consensus_config)? {
                result.append(&mut consensus_config::generate_consensus_upgrade_proposal(
                    consensus_config,
                    self.testnet,
                    if self.is_multi_step {
                        get_execution_hash(result)
                    } else {
                        "".to_owned().into_bytes()
                    },
                )?);
            }
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
            framework_release: Some(FrameworkReleaseConfig {
                bytecode_version: 5,
            }),
            gas_schedule: Some(aptos_gas::gen::current_gas_schedule()),
            version: None,
            feature_flags: None,
            consensus_config: Some(OnChainConsensusConfig::default()),
            is_multi_step: false,
            remote_endpoint: None,
        }
    }
}

pub fn get_execution_hash(result: &Vec<(String, String)>) -> Vec<u8> {
    if result.is_empty() {
        "vector::empty<u8>()".to_owned().into_bytes()
    } else {
        let temp_script_path = TempPath::new();
        temp_script_path.create_as_file().unwrap();
        let mut move_script_path = temp_script_path.path().to_path_buf();
        move_script_path.set_extension("move");
        std::fs::write(move_script_path.as_path(), result.last().unwrap().1.clone())
            .map_err(|err| {
                anyhow!(
                    "Failed to get execution hash: failed to write to file: {:?}",
                    err
                )
            })
            .unwrap();

        let (_, hash) = GenerateExecutionHash {
            script_path: Option::from(move_script_path),
        }
        .generate_hash()
        .unwrap();
        hash.to_vec()
    }
}
