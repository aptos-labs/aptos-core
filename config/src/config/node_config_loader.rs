// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{
        config_optimizer::ConfigOptimizer, config_sanitizer::ConfigSanitizer, utils::RootPath,
        Error, NodeConfig, PersistableConfig,
    },
    utils::get_genesis_txn,
};
use velor_types::{
    chain_id::ChainId,
    on_chain_config::OnChainConfig,
    state_store::state_key::StateKey,
    transaction::{Transaction, WriteSetPayload},
};
use serde_yaml::Value;
use std::path::Path;

/// A simple enum to represent the type of a node
/// as determined from the config file.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeType {
    Validator,
    ValidatorFullnode,
    PublicFullnode,
}

impl NodeType {
    pub fn is_validator(self) -> bool {
        self == NodeType::Validator
    }

    pub fn is_validator_fullnode(self) -> bool {
        self == NodeType::ValidatorFullnode
    }

    /// Returns the type of the node as determined by the node config
    pub fn extract_from_config(node_config: &NodeConfig) -> Self {
        // Validator nodes are trivial to detect
        if node_config.base.role.is_validator() {
            return NodeType::Validator;
        }

        // Otherwise, we must decipher between VFNs and PFNs
        // based on the presence of a VFN network.
        let vfn_network_found = node_config
            .full_node_networks
            .iter()
            .any(|network| network.network_id.is_vfn_network());
        if vfn_network_found {
            NodeType::ValidatorFullnode
        } else {
            NodeType::PublicFullnode
        }
    }
}

/// A simple node config loader that performs basic config
/// sanitization and post-processing.
pub struct NodeConfigLoader<P> {
    node_config_path: P,
}

impl<P: AsRef<Path>> NodeConfigLoader<P> {
    pub fn new(node_config_path: P) -> Self {
        Self { node_config_path }
    }

    /// Load the node config, validate the configuration options
    /// and process the config for the current environment.
    pub fn load_and_sanitize_config(&self) -> Result<NodeConfig, Error> {
        // Load the node config from disk
        let mut node_config = NodeConfig::load_config(&self.node_config_path)?;

        // Load the execution config
        let input_dir = RootPath::new(&self.node_config_path);
        node_config.execution.load_from_path(&input_dir)?;

        // Update the data directory. This needs to be done before
        // we optimize and sanitize the node configs (because some optimizers
        // rely on the data directory for file reading/writing).
        node_config.set_data_dir(node_config.get_data_dir().to_path_buf());

        // Optimize and sanitize the node config
        let local_config_yaml = get_local_config_yaml(&self.node_config_path)?;
        optimize_and_sanitize_node_config(&mut node_config, local_config_yaml)?;

        Ok(node_config)
    }
}

/// Return the node config file contents as a string
fn get_local_config_yaml<P: AsRef<Path>>(node_config_path: P) -> Result<Value, Error> {
    // Read the file contents into a string
    let local_config_yaml = NodeConfig::read_config_file(&node_config_path)?;

    // Parse the file contents as a yaml value
    let local_config_yaml = serde_yaml::from_str(&local_config_yaml).map_err(|error| {
        Error::Yaml(
            "Failed to parse the node config file into a YAML value".into(),
            error,
        )
    })?;

    Ok(local_config_yaml)
}

/// Extracts the node type and chain ID from the given node config
/// and genesis transaction. If the chain ID cannot be extracted,
/// None is returned.
fn extract_node_type_and_chain_id(node_config: &NodeConfig) -> (NodeType, Option<ChainId>) {
    // Get the node type from the node config
    let node_type = NodeType::extract_from_config(node_config);

    // Get the chain ID from the genesis transaction
    match get_chain_id(node_config) {
        Ok(chain_id) => (node_type, Some(chain_id)),
        Err(error) => {
            println!("Failed to extract the chain ID from the genesis transaction: {:?}! Continuing with None.", error);
            (node_type, None)
        },
    }
}

/// Optimize and sanitize the node config for the current environment
fn optimize_and_sanitize_node_config(
    node_config: &mut NodeConfig,
    local_config_yaml: Value,
) -> Result<(), Error> {
    // Extract the node type and chain ID from the node config
    let (node_type, chain_id) = extract_node_type_and_chain_id(node_config);

    // Print the extracted node type and chain ID
    println!(
        "Identified node type ({:?}) and chain ID ({:?}) from node config!",
        node_type, chain_id
    );

    // Optimize the node config
    NodeConfig::optimize(node_config, &local_config_yaml, node_type, chain_id)?;

    // Sanitize the node config
    NodeConfig::sanitize(node_config, node_type, chain_id)
}

/// Sanitize the node config for the current environment
pub fn sanitize_node_config(node_config: &mut NodeConfig) -> Result<(), Error> {
    // Extract the node type and chain ID from the node config
    let (node_type, chain_id) = extract_node_type_and_chain_id(node_config);

    // Sanitize the node config
    NodeConfig::sanitize(node_config, node_type, chain_id)
}

/// Get the chain ID for the node from the genesis transaction.
/// If the chain ID cannot be extracted, an error is returned.
fn get_chain_id(node_config: &NodeConfig) -> Result<ChainId, Error> {
    // TODO: can we make this less hacky?

    // Load the genesis transaction from disk
    let genesis_txn = get_genesis_txn(node_config).ok_or_else(|| {
        Error::InvariantViolation("The genesis transaction was not found!".to_string())
    })?;

    // Extract the chain ID from the genesis transaction
    match genesis_txn {
        Transaction::GenesisTransaction(WriteSetPayload::Direct(change_set)) => {
            let chain_id_state_key = StateKey::on_chain_config::<ChainId>()?;

            // Get the write op from the write set
            let write_set_mut = change_set.clone().write_set().clone().into_mut();
            let write_op = write_set_mut.get(&chain_id_state_key).ok_or_else(|| {
                Error::InvariantViolation(
                    "The genesis transaction does not contain the write op for the chain id!"
                        .into(),
                )
            })?;

            // Extract the chain ID from the write op
            let write_op_bytes = write_op.bytes().ok_or_else(|| Error::InvariantViolation(
                "The genesis transaction does not contain the correct write op for the chain ID!".into(),
            ))?;
            let chain_id = ChainId::deserialize_into_config(write_op_bytes).map_err(|error| {
                Error::InvariantViolation(format!(
                    "Failed to deserialize the chain ID: {:?}",
                    error
                ))
            })?;

            Ok(chain_id)
        },
        _ => Err(Error::InvariantViolation(format!(
            "The genesis transaction has the incorrect type: {:?}!",
            genesis_txn
        ))),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{node_config_loader::NodeType, BaseConfig, NetworkConfig, NodeConfig, RoleType},
        network_id::NetworkId,
    };

    #[test]
    fn test_node_type_from_validator_config() {
        // Create a validator node config
        let node_config = NodeConfig {
            base: BaseConfig {
                role: RoleType::Validator,
                ..Default::default()
            },
            ..Default::default()
        };

        // Verify the node type is correct
        assert_eq!(
            NodeType::extract_from_config(&node_config),
            NodeType::Validator
        );
    }

    #[test]
    fn test_node_type_from_vfn_config() {
        // Create a VFN node config
        let node_config = NodeConfig {
            base: BaseConfig {
                role: RoleType::FullNode,
                ..Default::default()
            },
            full_node_networks: vec![NetworkConfig::network_with_id(NetworkId::Vfn)],
            ..Default::default()
        };

        // Verify the node type is correct
        assert_eq!(
            NodeType::extract_from_config(&node_config),
            NodeType::ValidatorFullnode
        );
    }

    #[test]
    fn test_node_type_from_pfn_config() {
        // Create a PFN node config
        let node_config = NodeConfig {
            base: BaseConfig {
                role: RoleType::FullNode,
                ..Default::default()
            },
            full_node_networks: vec![NetworkConfig::network_with_id(NetworkId::Public)],
            ..Default::default()
        };

        // Verify the node type is correct
        assert_eq!(
            NodeType::extract_from_config(&node_config),
            NodeType::PublicFullnode
        );
    }
}
