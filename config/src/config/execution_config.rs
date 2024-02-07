// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::WaypointConfig;
use crate::config::{
    config_sanitizer::ConfigSanitizer, node_config_loader::NodeType,
    transaction_filter_type::Filter, utils::RootPath, Error, NodeConfig,
};
use aptos_types::{chain_id::ChainId, transaction::Transaction};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

const GENESIS_DEFAULT: &str = "genesis.blob";

#[derive(Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExecutionConfig {
    #[serde(skip)]
    /// For testing purposes, the ability to add a genesis transaction directly
    pub genesis: Option<Transaction>,
    /// Location of the genesis file
    pub genesis_file_location: PathBuf,
    /// Number of threads to run execution
    pub concurrency_level: u16,
    /// Number of threads to read proofs
    pub num_proof_reading_threads: u16,
    /// Enables paranoid mode for types, which adds extra runtime VM checks
    pub paranoid_type_verification: bool,
    /// Enables paranoid mode for hot potatoes, which adds extra runtime VM checks
    pub paranoid_hot_potato_verification: bool,
    /// Enables enhanced metrics around processed transactions
    pub processed_transactions_detailed_counters: bool,
    /// Enables filtering of transactions before they are sent to execution
    pub transaction_filter: Filter,
    /// Used during DB bootstrapping
    pub genesis_waypoint: Option<WaypointConfig>,
}

impl std::fmt::Debug for ExecutionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExecutionConfig {{ genesis: ")?;
        if self.genesis.is_some() {
            write!(f, "Some(...)")?;
        } else {
            write!(f, "None")?;
        }
        write!(
            f,
            ", genesis_file_location: {:?} ",
            self.genesis_file_location
        )
    }
}

impl Default for ExecutionConfig {
    fn default() -> ExecutionConfig {
        ExecutionConfig {
            genesis: None,
            genesis_file_location: PathBuf::new(),
            // Parallel execution by default.
            concurrency_level: 8,
            num_proof_reading_threads: 32,
            paranoid_type_verification: true,
            paranoid_hot_potato_verification: true,
            processed_transactions_detailed_counters: false,
            transaction_filter: Filter::empty(),
            genesis_waypoint: None,
        }
    }
}

impl ExecutionConfig {
    pub fn load_from_path(&mut self, root_dir: &RootPath) -> Result<(), Error> {
        if !self.genesis_file_location.as_os_str().is_empty() {
            // Ensure the genesis file exists
            let genesis_path = root_dir.full_path(&self.genesis_file_location);
            if !genesis_path.exists() {
                return Err(Error::Unexpected(format!(
                    "The genesis file could not be found! Ensure the given path is correct: {:?}",
                    genesis_path.display()
                )));
            }

            // Open the genesis file and read the bytes
            let mut file = File::open(&genesis_path).map_err(|error| {
                Error::Unexpected(format!(
                    "Failed to open the genesis file: {:?}. Error: {:?}",
                    genesis_path.display(),
                    error
                ))
            })?;
            let mut buffer = vec![];
            file.read_to_end(&mut buffer).map_err(|error| {
                Error::Unexpected(format!(
                    "Failed to read the genesis file into a buffer: {:?}. Error: {:?}",
                    genesis_path.display(),
                    error
                ))
            })?;

            // Deserialize the genesis file and store it
            let genesis = bcs::from_bytes(&buffer).map_err(|error| {
                Error::Unexpected(format!(
                    "Failed to BCS deserialize the genesis file: {:?}. Error: {:?}",
                    genesis_path.display(),
                    error
                ))
            })?;
            self.genesis = Some(genesis);
        }

        Ok(())
    }

    pub fn save_to_path(&mut self, root_dir: &RootPath) -> Result<(), Error> {
        if let Some(genesis) = &self.genesis {
            if self.genesis_file_location.as_os_str().is_empty() {
                self.genesis_file_location = PathBuf::from(GENESIS_DEFAULT);
            }
            let path = root_dir.full_path(&self.genesis_file_location);
            let mut file = File::create(path).map_err(|e| Error::IO("genesis".into(), e))?;
            let data = bcs::to_bytes(&genesis).map_err(|e| Error::BCS("genesis", e))?;
            file.write_all(&data)
                .map_err(|e| Error::IO("genesis".into(), e))?;
        }
        Ok(())
    }
}

impl ConfigSanitizer for ExecutionConfig {
    fn sanitize(
        node_config: &NodeConfig,
        _node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();
        let execution_config = &node_config.execution;

        // If this is a mainnet node, ensure that additional verifiers are enabled
        if let Some(chain_id) = chain_id {
            if chain_id.is_mainnet() {
                if !execution_config.paranoid_hot_potato_verification {
                    return Err(Error::ConfigSanitizerFailed(
                        sanitizer_name,
                        "paranoid_hot_potato_verification must be enabled for mainnet nodes!"
                            .into(),
                    ));
                }
                if !execution_config.paranoid_type_verification {
                    return Err(Error::ConfigSanitizerFailed(
                        sanitizer_name,
                        "paranoid_type_verification must be enabled for mainnet nodes!".into(),
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use aptos_temppath::TempPath;
    use aptos_types::{
        transaction::{ChangeSet, Transaction, WriteSetPayload},
        write_set::WriteSetMut,
    };

    #[test]
    fn test_sanitize_valid_execution_config() {
        // Create a node config with a valid execution config
        let node_config = NodeConfig {
            execution: ExecutionConfig {
                paranoid_hot_potato_verification: true,
                paranoid_type_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it succeeds
        ExecutionConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::mainnet()))
            .unwrap();
    }

    #[test]
    fn test_sanitize_hot_potato_mainnet() {
        // Create a node config with missing paranoid_hot_potato_verification on mainnet
        let node_config = NodeConfig {
            execution: ExecutionConfig {
                paranoid_hot_potato_verification: false,
                paranoid_type_verification: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error =
            ExecutionConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::mainnet()))
                .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_sanitize_paranoid_type_mainnet() {
        // Create a node config with missing paranoid_type_verification on mainnet
        let node_config = NodeConfig {
            execution: ExecutionConfig {
                paranoid_hot_potato_verification: true,
                paranoid_type_verification: false,
                ..Default::default()
            },
            ..Default::default()
        };

        // Sanitize the config and verify that it fails
        let error =
            ExecutionConfig::sanitize(&node_config, NodeType::Validator, Some(ChainId::mainnet()))
                .unwrap_err();
        assert!(matches!(error, Error::ConfigSanitizerFailed(_, _)));
    }

    #[test]
    fn test_no_genesis() {
        let (mut config, path) = generate_config();
        assert_eq!(config.genesis, None);
        let root_dir = RootPath::new_path(path.path());
        let result = config.load_from_path(&root_dir);
        assert!(result.is_ok());
        assert_eq!(config.genesis_file_location, PathBuf::new());
    }

    #[test]
    fn test_some_and_load_genesis() {
        let fake_genesis = Transaction::GenesisTransaction(WriteSetPayload::Direct(
            ChangeSet::new(WriteSetMut::new(vec![]).freeze().unwrap(), vec![]),
        ));
        let (mut config, path) = generate_config();
        config.genesis = Some(fake_genesis.clone());
        let root_dir = RootPath::new_path(path.path());
        config.save_to_path(&root_dir).expect("Unable to save");
        // Verifies some without path
        assert_eq!(config.genesis_file_location, PathBuf::from(GENESIS_DEFAULT));

        config.genesis = None;
        let result = config.load_from_path(&root_dir);
        assert!(result.is_ok());
        assert_eq!(config.genesis, Some(fake_genesis));
    }

    fn generate_config() -> (ExecutionConfig, TempPath) {
        let temp_dir = TempPath::new();
        temp_dir.create_as_dir().expect("error creating tempdir");
        let execution_config = ExecutionConfig::default();
        (execution_config, temp_dir)
    }
}
