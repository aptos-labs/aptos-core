// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::WaypointConfig;
use crate::config::{
    config_optimizer::ConfigOptimizer, config_sanitizer::ConfigSanitizer,
    node_config_loader::NodeType, utils::RootPath, Error, NodeConfig,
};
use aptos_transactions_filter::transaction_matcher::Filter;
use aptos_types::{chain_id::ChainId, transaction::Transaction, waypoint::Waypoint};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    str::FromStr,
};

// Default execution concurrency level
pub const DEFAULT_EXECUTION_CONCURRENCY_LEVEL: u16 = 32;

// Genesis constants
const GENESIS_BLOB_FILENAME: &str = "genesis.blob";
const GENESIS_VERSION: u64 = 0;
const MAINNET_GENESIS_WAYPOINT: &str =
    "0:6072b68a942aace147e0655c5704beaa255c84a7829baa4e72a500f1516584c4";
const TESTNET_GENESIS_WAYPOINT: &str =
    "0:4b56f15c1dcef7f9f3eb4b4798c0cba0f1caacc0d35f1c80ad9b7a21f1f8b454";

#[derive(Clone, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ExecutionConfig {
    #[serde(skip)]
    /// For testing purposes, the ability to add a genesis transaction directly
    pub genesis: Option<Transaction>,
    /// Location of the genesis file
    pub genesis_file_location: PathBuf,
    /// Number of threads to run execution.
    /// If 0, we use min of (num of cores/2, DEFAULT_CONCURRENCY_LEVEL) as default concurrency level
    pub concurrency_level: u16,
    /// Number of threads to read proofs
    pub num_proof_reading_threads: u16,
    /// Enables paranoid mode for types, which adds extra runtime VM checks
    pub paranoid_type_verification: bool,
    /// Enabled discarding blocks that fail execution due to BlockSTM/VM issue.
    pub discard_failed_blocks: bool,
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
            // use min of (num of cores/2, DEFAULT_CONCURRENCY_LEVEL) as default concurrency level
            concurrency_level: 0,
            num_proof_reading_threads: 32,
            paranoid_type_verification: true,
            paranoid_hot_potato_verification: true,
            discard_failed_blocks: false,
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
                self.genesis_file_location = PathBuf::from(GENESIS_BLOB_FILENAME);
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

impl ConfigOptimizer for ExecutionConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        local_config_yaml: &Value,
        _node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        let execution_config = &mut node_config.execution;
        let local_execution_config_yaml = &local_config_yaml["execution"];

        // If the base config has a non-genesis waypoint, we should automatically
        // inject the genesis waypoint into the execution config (if it doesn't exist).
        // We do this for testnet and mainnet only (as they are long lived networks).
        if node_config.base.waypoint.waypoint().version() != GENESIS_VERSION
            && execution_config.genesis_waypoint.is_none()
            && local_execution_config_yaml["genesis_waypoint"].is_null()
        {
            // Determine the genesis waypoint string to use
            let genesis_waypoint_str = match chain_id {
                Some(chain_id) => {
                    if chain_id.is_mainnet() {
                        MAINNET_GENESIS_WAYPOINT
                    } else if chain_id.is_testnet() {
                        TESTNET_GENESIS_WAYPOINT
                    } else {
                        return Ok(false); // Return early (this is not testnet or mainnet)
                    }
                },
                None => return Ok(false), // Return early (no chain ID was specified!)
            };

            // Construct a genesis waypoint from the string
            let genesis_waypoint = match Waypoint::from_str(genesis_waypoint_str) {
                Ok(waypoint) => waypoint,
                Err(error) => panic!(
                    "Invalid genesis waypoint string: {:?}. Error: {:?}",
                    genesis_waypoint_str, error
                ),
            };
            let genesis_waypoint_config = WaypointConfig::FromConfig(genesis_waypoint);

            // Inject the genesis waypoint into the execution config
            execution_config.genesis_waypoint = Some(genesis_waypoint_config);

            return Ok(true); // The config was modified
        }

        Ok(false) // The config was not modified
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
    use std::{assert_eq, matches, vec};

    // Useful test constants
    const GENESIS_WAYPOINT: &str =
        "0:00000000002aace147e0655c5704beaa255c84a7829baa4e72a5000000000000";
    const NON_GENESIS_WAYPOINT: &str =
        "100:aaaaaaaaaa2aace147e0655c5704beaa255c84a7829baa4e72a500aaaaaaaaaa";

    #[test]
    fn test_optimize_execution_config_genesis() {
        // Create a default node config
        let mut node_config = NodeConfig::default();

        // Verify the execution config does not have a genesis waypoint
        assert!(&node_config.execution.genesis_waypoint.is_none());

        // Inject a genesis waypoint into the base config
        let genesis_waypoint = Waypoint::from_str(GENESIS_WAYPOINT).unwrap();
        node_config.base.waypoint = WaypointConfig::FromConfig(genesis_waypoint);

        // Optimize the config and verify that no modifications are made
        let modified_config = ExecutionConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::Validator,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(!modified_config);
    }

    #[test]
    fn test_optimize_execution_config_non_genesis_mainnet() {
        // Create a default node config
        let mut node_config = NodeConfig::default();

        // Verify the execution config does not have a genesis waypoint
        assert!(&node_config.execution.genesis_waypoint.is_none());

        // Inject a non-genesis waypoint into the base config
        let non_genesis_waypoint = Waypoint::from_str(NON_GENESIS_WAYPOINT).unwrap();
        node_config.base.waypoint = WaypointConfig::FromConfig(non_genesis_waypoint);

        // Optimize the config for mainnet and verify modifications are made
        let modified_config = ExecutionConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::Validator,
            Some(ChainId::mainnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that the mainnet genesis waypoint was injected into the execution config
        let expected_genesis_waypoint =
            WaypointConfig::FromConfig(Waypoint::from_str(MAINNET_GENESIS_WAYPOINT).unwrap());
        assert_eq!(
            &node_config.execution.genesis_waypoint,
            &Some(expected_genesis_waypoint)
        );
    }

    #[test]
    fn test_optimize_execution_config_non_genesis_testnet() {
        // Create a default node config
        let mut node_config = NodeConfig::default();

        // Verify the execution config does not have a genesis waypoint
        assert!(&node_config.execution.genesis_waypoint.is_none());

        // Inject a non-genesis waypoint into the base config
        let non_genesis_waypoint = Waypoint::from_str(NON_GENESIS_WAYPOINT).unwrap();
        node_config.base.waypoint = WaypointConfig::FromConfig(non_genesis_waypoint);

        // Optimize the config for testnet and verify modifications are made
        let modified_config = ExecutionConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::PublicFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that the testnet genesis waypoint was injected into the execution config
        let expected_genesis_waypoint =
            WaypointConfig::FromConfig(Waypoint::from_str(TESTNET_GENESIS_WAYPOINT).unwrap());
        assert_eq!(
            &node_config.execution.genesis_waypoint,
            &Some(expected_genesis_waypoint)
        );
    }

    #[test]
    fn test_optimize_execution_config_skipped() {
        // Create a default node config
        let mut node_config = NodeConfig::default();

        // Verify the execution config does not have a genesis waypoint
        assert!(&node_config.execution.genesis_waypoint.is_none());

        // Inject a non-genesis waypoint into the base config
        let non_genesis_waypoint = Waypoint::from_str(NON_GENESIS_WAYPOINT).unwrap();
        node_config.base.waypoint = WaypointConfig::FromConfig(non_genesis_waypoint);

        // Optimize the config for a local network and verify that no modifications are made
        let modified_config = ExecutionConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::Validator,
            Some(ChainId::new(100)), // Neither testnet or mainnet
        )
        .unwrap();
        assert!(!modified_config);

        // Create a local config YAML with an execution config explicitly setting the genesis waypoint
        let local_config_yaml = serde_yaml::from_str(
            r#"
            execution:
                genesis_waypoint: "0:aaaaaaaa"
            "#,
        )
        .unwrap();

        // Optimize the config for mainnet and verify that no modifications are made
        let modified_config = ExecutionConfig::optimize(
            &mut node_config,
            &local_config_yaml, // The local config with an explicit genesis waypoint
            NodeType::Validator,
            Some(ChainId::mainnet()),
        )
        .unwrap();
        assert!(!modified_config);
    }

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
        assert_eq!(
            config.genesis_file_location,
            PathBuf::from(GENESIS_BLOB_FILENAME)
        );

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
