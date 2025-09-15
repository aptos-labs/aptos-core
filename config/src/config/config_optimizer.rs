// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    ConsensusConfig, ConsensusObserverConfig, Identity, IdentityFromConfig, IdentitySource,
    IndexerGrpcConfig, StorageConfig,
};
use crate::{
    config::{
        node_config_loader::NodeType, utils::get_config_name, AdminServiceConfig, Error,
        ExecutionConfig, IndexerConfig, InspectionServiceConfig, LoggerConfig, MempoolConfig,
        NodeConfig, Peer, PeerRole, PeerSet, StateSyncConfig,
    },
    network_id::NetworkId,
};
use aptos_crypto::{x25519, ValidCryptoMaterialStringExt};
use aptos_types::{chain_id::ChainId, network_address::NetworkAddress, PeerId};
use maplit::hashset;
use serde_yaml::Value;
use std::{collections::HashMap, str::FromStr};

// Useful optimizer constants
const OPTIMIZER_STRING: &str = "Optimizer";
const ALL_NETWORKS_OPTIMIZER_NAME: &str = "AllNetworkConfigOptimizer";
const PUBLIC_NETWORK_OPTIMIZER_NAME: &str = "PublicNetworkConfigOptimizer";
const VALIDATOR_NETWORK_OPTIMIZER_NAME: &str = "ValidatorNetworkConfigOptimizer";

const IDENTITY_KEY_FILE: &str = "ephemeral_identity_key";

// Mainnet seed peers. Each seed peer entry is a tuple
// of (account address, public key, network address).
const MAINNET_SEED_PEERS: [(&str, &str, &str); 1] = [(
    "568fdb6acf26aae2a84419108ff13baa3ebf133844ef18e23a9f47b5af16b698",
    "0x003cc2ed36e7d486539ac2c411b48d962f1ef17d884c3a7109cad43f16bd5008",
    "/dns/node1.cloud-b.mainnet.aptoslabs.com/tcp/6182/noise-ik/0x003cc2ed36e7d486539ac2c411b48d962f1ef17d884c3a7109cad43f16bd5008/handshake/0",
)];

// Testnet seed peers. Each seed peer entry is a tuple
// of (account address, public key, network address).
const TESTNET_SEED_PEERS: [(&str, &str, &str); 4] = [
    (
        "31e55012a7d439dcd16fee0509cd5855c1fbdc62057ba7fac3f7c88f5453dd8e",
        "0x87bb19b02580b7e2a91a8e9342ec77ffd8f3ad967f54e77b22aaf558c5c11755",
        "/dns/seed0.testnet.aptoslabs.com/tcp/6182/noise-ik/0x87bb19b02580b7e2a91a8e9342ec77ffd8f3ad967f54e77b22aaf558c5c11755/handshake/0",
    ),
    (
        "116176e2af223a8b7f8db80dc52f7a423b4d7f8c0553a1747e92ef58849aff4f",
        "0xc2f24389f31c9c18d2ceb69d153ad9299e0ea7bbd66f457e0a28ef41c77c2b64",
        "/dns/seed1.testnet.aptoslabs.com/tcp/6182/noise-ik/0xc2f24389f31c9c18d2ceb69d153ad9299e0ea7bbd66f457e0a28ef41c77c2b64/handshake/0",
    ),
    (
        "12000330d7cd8a748f46c25e6ce5d236a27e13d0b510d4516ac84ecc5fddd002",
        "0x171c661e5b785283978a74eafc52a906e68c73ae78119737b92f93507c753933",
        "/dns/seed2.testnet.aptoslabs.com/tcp/6182/noise-ik/0x171c661e5b785283978a74eafc52a906e68c73ae78119737b92f93507c753933/handshake/0",
    ),
    (
        "03c04549114877c55f45649aba48ac0a4ff086ab7bdce3b8cc8d3d9947bc0d99",
        "0xafc38bf177bd825326a1c314748612137d2b35dae6472932806806a32c23174a",
        "/dns/seed3.testnet.aptoslabs.com/tcp/6182/noise-ik/0xafc38bf177bd825326a1c314748612137d2b35dae6472932806806a32c23174a/handshake/0",
    ),
];

/// A trait for optimizing node configs (and their sub-configs) by tweaking
/// config values based on node types, chain IDs and compiler features.
///
/// Note: The config optimizer respects the following order precedence when
/// determining whether or not to optimize a value:
/// 1. If a config value has been set in the local config file, that value
///    should be used (and the optimizer should not override it).
/// 2. If a config value has not been set in the local config file, the
///    optimizer may set the value (but, it is not required to do so).
/// 3. Finally, if the config optimizer chooses not to set a value, the default
///    value is used (as defined in the default implementation).
pub trait ConfigOptimizer {
    /// Get the name of the optimizer (e.g., for logging)
    fn get_optimizer_name() -> String {
        let config_name = get_config_name::<Self>().to_string();
        config_name + OPTIMIZER_STRING
    }

    /// Optimize the node config according to the given node type and chain ID
    /// and return true iff the config was modified.
    ///
    /// Note: the `local_config_yaml` contains the raw YAML string of the node
    /// config as provided by the user. This is used to check if a value
    /// should not be optimized/modified (as it has been set by the user).
    fn optimize(
        _node_config: &mut NodeConfig,
        _local_config_yaml: &Value,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        unimplemented!("optimize() must be implemented for each optimizer!");
    }
}

impl ConfigOptimizer for NodeConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        local_config_yaml: &Value,
        node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        // If config optimization is disabled, don't do anything!
        if node_config.node_startup.skip_config_optimizer {
            return Ok(false);
        }

        // Optimize only the relevant sub-configs
        let mut optimizers_with_modifications = vec![];
        if AdminServiceConfig::optimize(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(AdminServiceConfig::get_optimizer_name());
        }
        if ConsensusConfig::optimize(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(ConsensusConfig::get_optimizer_name());
        }
        if ConsensusObserverConfig::optimize(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(ConsensusObserverConfig::get_optimizer_name());
        }
        if ExecutionConfig::optimize(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(ExecutionConfig::get_optimizer_name());
        }
        if IndexerConfig::optimize(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(IndexerConfig::get_optimizer_name());
        }
        if IndexerGrpcConfig::optimize(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(IndexerGrpcConfig::get_optimizer_name());
        }
        if InspectionServiceConfig::optimize(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(InspectionServiceConfig::get_optimizer_name());
        }
        if LoggerConfig::optimize(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(LoggerConfig::get_optimizer_name());
        }
        if MempoolConfig::optimize(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(MempoolConfig::get_optimizer_name());
        }
        if StateSyncConfig::optimize(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(StateSyncConfig::get_optimizer_name());
        }
        if StorageConfig::optimize(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(StorageConfig::get_optimizer_name());
        }
        if optimize_all_network_configs(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(ALL_NETWORKS_OPTIMIZER_NAME.to_string());
        }
        if optimize_public_network_config(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(PUBLIC_NETWORK_OPTIMIZER_NAME.to_string());
        }
        if optimize_validator_network_config(node_config, local_config_yaml, node_type, chain_id)? {
            optimizers_with_modifications.push(VALIDATOR_NETWORK_OPTIMIZER_NAME.to_string());
        }

        // Return true iff any config modifications were made
        Ok(!optimizers_with_modifications.is_empty())
    }
}

/// Optimizes all network configs according to the node type and chain ID
fn optimize_all_network_configs(
    node_config: &mut NodeConfig,
    _local_config_yaml: &Value,
    _node_type: NodeType,
    _chain_id: Option<ChainId>,
) -> Result<bool, Error> {
    let mut modified_config = false;

    // Set the listener address and prepare the node identities for the validator network
    if let Some(validator_network) = &mut node_config.validator_network {
        validator_network.set_listen_address_and_prepare_identity()?;
        modified_config = true;
    }

    // Set the listener address and prepare the node identities for the fullnode networks
    for fullnode_network in &mut node_config.full_node_networks {
        fullnode_network.set_listen_address_and_prepare_identity()?;
        modified_config = true;
    }

    Ok(modified_config)
}

/// Optimize the public network config according to the node type and chain ID
fn optimize_public_network_config(
    node_config: &mut NodeConfig,
    local_config_yaml: &Value,
    node_type: NodeType,
    chain_id: Option<ChainId>,
) -> Result<bool, Error> {
    // We only need to optimize the public network config for VFNs and PFNs
    if node_type.is_validator() {
        return Ok(false);
    }

    // Add seeds to the public network config
    let mut modified_config = false;
    for (index, fullnode_network_config) in node_config.full_node_networks.iter_mut().enumerate() {
        let local_network_config_yaml = &local_config_yaml["full_node_networks"][index];

        // Optimize the public network configs
        if fullnode_network_config.network_id == NetworkId::Public {
            // Only add seeds to testnet and mainnet (as they are long living networks)
            if local_network_config_yaml["seeds"].is_null() {
                if let Some(chain_id) = chain_id {
                    if chain_id.is_testnet() {
                        fullnode_network_config.seeds =
                            create_seed_peers(TESTNET_SEED_PEERS.into())?;
                        modified_config = true;
                    } else if chain_id.is_mainnet() {
                        fullnode_network_config.seeds =
                            create_seed_peers(MAINNET_SEED_PEERS.into())?;
                        modified_config = true;
                    }
                }
            }

            // If the identity key was not set in the config, attempt to
            // load it from disk. Otherwise, save the already generated
            // one to disk (for future runs).
            if let Identity::FromConfig(IdentityFromConfig {
                source: IdentitySource::AutoGenerated,
                key: config_key,
                ..
            }) = &fullnode_network_config.identity
            {
                let path = node_config.storage.dir().join(IDENTITY_KEY_FILE);
                if let Some(loaded_identity) = Identity::load_identity(&path)? {
                    fullnode_network_config.identity = loaded_identity;
                } else {
                    Identity::save_private_key(&path, &config_key.private_key())?;
                }
            }
        }
    }

    Ok(modified_config)
}

/// Optimize the validator network config according to the node type and chain ID
fn optimize_validator_network_config(
    node_config: &mut NodeConfig,
    local_config_yaml: &Value,
    _node_type: NodeType,
    _chain_id: Option<ChainId>,
) -> Result<bool, Error> {
    let mut modified_config = false;
    if let Some(validator_network_config) = &mut node_config.validator_network {
        let local_network_config_yaml = &local_config_yaml["validator_network_config"];

        // We must override the network ID to be a validator
        // network ID (as the config defaults to a public network ID).
        if local_network_config_yaml["network_id"].is_null() {
            validator_network_config.network_id = NetworkId::Validator;
            modified_config = true;
        }

        // We must enable mutual authentication for the validator network
        if local_network_config_yaml["mutual_authentication"].is_null() {
            validator_network_config.mutual_authentication = true;
            modified_config = true;
        }
    }

    Ok(modified_config)
}

/// Creates and returns a set of seed peers from the given entries
fn create_seed_peers(seed_peer_entries: Vec<(&str, &str, &str)>) -> Result<PeerSet, Error> {
    // Create a map of seed peers
    let mut seed_peers = HashMap::new();

    // Add the seed peers
    for (account_address, public_key, network_address) in seed_peer_entries {
        let (peer_address, peer) = build_seed_peer(account_address, public_key, network_address)?;
        seed_peers.insert(peer_address, peer);
    }

    Ok(seed_peers)
}

/// Builds a seed peer using the specified peer information
fn build_seed_peer(
    account_address_hex: &str,
    public_key_hex: &str,
    network_address_str: &str,
) -> Result<(PeerId, Peer), Error> {
    // Parse the account address
    let account_address = PeerId::from_hex(account_address_hex).map_err(|error| {
        Error::Unexpected(format!(
            "Failed to parse peer account address: {:?}. Error: {:?}",
            account_address_hex, error
        ))
    })?;

    // Parse the x25519 public key
    let public_key = x25519::PublicKey::from_encoded_string(public_key_hex).map_err(|error| {
        Error::Unexpected(format!(
            "Failed to parse peer public key: {:?}. Error: {:?}",
            public_key_hex, error
        ))
    })?;

    // Parse the network address string
    let network_address = NetworkAddress::from_str(network_address_str).map_err(|error| {
        Error::Unexpected(format!(
            "Failed to parse peer network address: {:?}. Error: {:?}",
            network_address_str, error
        ))
    })?;

    // Build the peer struct
    let peer = Peer {
        addresses: vec![network_address],
        keys: hashset! {public_key},
        role: PeerRole::Upstream,
    };

    // Return the account address and peer
    Ok((account_address, peer))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{
            node_startup_config::NodeStartupConfig, NetworkConfig, StorageConfig, WaypointConfig,
        },
        network_id::NetworkId,
    };
    use aptos_crypto::{Uniform, ValidCryptoMaterial};
    use aptos_types::{account_address::AccountAddress, waypoint::Waypoint};
    use rand::rngs::OsRng;
    use std::{io::Write, path::PathBuf};
    use tempfile::{tempdir, NamedTempFile};

    fn setup_storage_config_with_temp_dir() -> (StorageConfig, PathBuf) {
        let temp_dir = tempdir().unwrap();
        let mut storage_config = StorageConfig::default();
        storage_config.dir = temp_dir.path().to_path_buf();
        (storage_config, temp_dir.into_path())
    }

    #[test]
    fn test_disable_optimizer() {
        // Create a default node config (with optimization enabled)
        let mut node_config = NodeConfig::default();

        // Set the base waypoint config
        node_config.base.waypoint = WaypointConfig::FromConfig(Waypoint::default());

        // Optimize the node config for mainnet VFNs and verify modifications are made
        let modified_config = NodeConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str(
                r#"
            storage:
              rocksdb_configs:
                enable_storage_sharding: true
            "#,
            )
            .unwrap(),
            NodeType::ValidatorFullnode,
            Some(ChainId::mainnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Create a node config with the optimizer disabled
        let mut node_config = NodeConfig {
            node_startup: NodeStartupConfig {
                skip_config_optimizer: true,
                ..Default::default()
            },
            ..Default::default()
        };

        // Optimize the node config for mainnet VFNs and verify no modifications are made
        let modified_config = NodeConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config
            NodeType::ValidatorFullnode,
            Some(ChainId::mainnet()),
        )
        .unwrap();
        assert!(!modified_config);
    }

    #[test]
    fn test_optimize_public_network_config_mainnet() {
        // Create a public network config with no seeds
        let mut node_config = NodeConfig {
            storage: setup_storage_config_with_temp_dir().0,
            full_node_networks: vec![NetworkConfig {
                network_id: NetworkId::Public,
                seeds: HashMap::new(),
                ..Default::default()
            }],
            ..Default::default()
        };

        // Optimize the public network config and verify modifications are made
        let modified_config = optimize_public_network_config(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config
            NodeType::ValidatorFullnode,
            Some(ChainId::mainnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that the mainnet seed peers have been added to the config
        let public_network_config = &node_config.full_node_networks[0];
        let public_seeds = &public_network_config.seeds;
        assert_eq!(public_seeds.len(), MAINNET_SEED_PEERS.len());

        // Verify that the seed peers contain the expected values
        for (account_address, public_key, network_address) in MAINNET_SEED_PEERS {
            // Fetch the seed peer
            let seed_peer = public_seeds
                .get(&AccountAddress::from_hex(account_address).unwrap())
                .unwrap();

            // Verify the seed peer properties
            assert_eq!(seed_peer.role, PeerRole::Upstream);
            assert!(seed_peer
                .addresses
                .contains(&NetworkAddress::from_str(network_address).unwrap()));
            assert!(seed_peer
                .keys
                .contains(&x25519::PublicKey::from_encoded_string(public_key).unwrap()));
        }
    }

    #[test]
    fn test_optimize_public_network_config_testnet() {
        // Create a public network config with no seeds
        let mut node_config = NodeConfig {
            storage: setup_storage_config_with_temp_dir().0,
            full_node_networks: vec![NetworkConfig {
                network_id: NetworkId::Public,
                seeds: HashMap::new(),
                ..Default::default()
            }],
            ..Default::default()
        };

        // Optimize the public network config and verify modifications are made
        let modified_config = optimize_public_network_config(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config
            NodeType::PublicFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that the testnet seed peers have been added to the config
        let public_network_config = &node_config.full_node_networks[0];
        let public_seeds = &public_network_config.seeds;
        assert_eq!(public_seeds.len(), TESTNET_SEED_PEERS.len());

        // Verify that the seed peers contain the expected values
        for (account_address, public_key, network_address) in TESTNET_SEED_PEERS {
            // Fetch the seed peer
            let seed_peer = public_seeds
                .get(&AccountAddress::from_hex(account_address).unwrap())
                .unwrap();

            // Verify the seed peer properties
            assert_eq!(seed_peer.role, PeerRole::Upstream);
            assert!(seed_peer
                .addresses
                .contains(&NetworkAddress::from_str(network_address).unwrap()));
            assert!(seed_peer
                .keys
                .contains(&x25519::PublicKey::from_encoded_string(public_key).unwrap()));
        }
    }

    #[test]
    fn test_optimize_public_network_config_no_override() {
        // Create a public network config
        let mut node_config = NodeConfig {
            storage: setup_storage_config_with_temp_dir().0,
            full_node_networks: vec![NetworkConfig {
                network_id: NetworkId::Public,
                seeds: HashMap::new(),
                ..Default::default()
            }],
            ..Default::default()
        };

        // Create a local config with the public network having seed entries
        let local_config_yaml = serde_yaml::from_str(
            r#"
            full_node_networks:
                - network_id: "Public"
                  seeds:
                      bb14af025d226288a3488b4433cf5cb54d6a710365a2d95ac6ffbd9b9198a86a:
                          addresses:
                              - "/dns4/pfn0.node.devnet.aptoslabs.com/tcp/6182/noise-ik/bb14af025d226288a3488b4433cf5cb54d6a710365a2d95ac6ffbd9b9198a86a/handshake/0"
                          role: "Upstream"
            "#,
        )
            .unwrap();

        // Optimize the public network config and verify no modifications are made
        let modified_config = optimize_public_network_config(
            &mut node_config,
            &local_config_yaml,
            NodeType::PublicFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(!modified_config);
    }

    #[test]
    fn test_optimize_public_network_config_no_modifications() {
        // Create a public network config with no seeds
        let mut node_config = NodeConfig {
            storage: setup_storage_config_with_temp_dir().0,
            full_node_networks: vec![NetworkConfig {
                network_id: NetworkId::Public,
                seeds: HashMap::new(),
                ..Default::default()
            }],
            ..Default::default()
        };

        // Optimize the public network config and verify no modifications
        // are made (the node is a validator).
        let modified_config = optimize_public_network_config(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config
            NodeType::Validator,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(!modified_config);

        // Optimize the public network config and verify no modifications
        // are made (the chain ID is not testnet or mainnet).
        let modified_config = optimize_public_network_config(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config
            NodeType::PublicFullnode,
            Some(ChainId::test()),
        )
        .unwrap();
        assert!(!modified_config);
    }

    #[test]
    fn test_optimize_validator_network_config() {
        // Create a validator network config with incorrect defaults
        let mut node_config = NodeConfig {
            validator_network: Some(NetworkConfig {
                network_id: NetworkId::Public,
                mutual_authentication: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Optimize the validator network config and verify modifications are made
        let modified_config = optimize_validator_network_config(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config
            NodeType::Validator,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that the network ID and mutual authentication have been changed
        let validator_network = node_config.validator_network.unwrap();
        assert_eq!(validator_network.network_id, NetworkId::Validator);
        assert!(validator_network.mutual_authentication);
    }

    #[test]
    fn test_optimize_validator_config_no_override() {
        // Create a validator network config with incorrect defaults
        let mut node_config = NodeConfig {
            validator_network: Some(NetworkConfig {
                network_id: NetworkId::Public,
                mutual_authentication: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Create a local config with the network ID overridden
        let local_config_yaml = serde_yaml::from_str(
            r#"
            validator_network_config:
                network_id: "Public"
            "#,
        )
        .unwrap();

        // Optimize the validator network config and verify modifications are made
        let modified_config = optimize_validator_network_config(
            &mut node_config,
            &local_config_yaml,
            NodeType::Validator,
            Some(ChainId::mainnet()),
        )
        .unwrap();
        assert!(modified_config);

        // Verify that the network ID has not changed but that
        // mutual authentication has been enabled.
        let validator_network = node_config.validator_network.unwrap();
        assert_eq!(validator_network.network_id, NetworkId::Public);
        assert!(validator_network.mutual_authentication);
    }

    #[test]
    fn test_optimize_validator_config_no_modifications() {
        // Create a validator network config with incorrect defaults
        let mut node_config = NodeConfig {
            validator_network: Some(NetworkConfig {
                network_id: NetworkId::Public,
                mutual_authentication: false,
                ..Default::default()
            }),
            ..Default::default()
        };

        // Create a local config with the network ID and mutual authentication set
        let local_config_yaml = serde_yaml::from_str(
            r#"
            validator_network_config:
                network_id: "Public"
                mutual_authentication: false
            "#,
        )
        .unwrap();

        // Optimize the validator network config and verify no modifications are made
        let modified_config = optimize_validator_network_config(
            &mut node_config,
            &local_config_yaml,
            NodeType::Validator,
            Some(ChainId::mainnet()),
        )
        .unwrap();
        assert!(!modified_config);
    }

    #[test]
    fn test_load_identity_nonexistent() {
        let path = PathBuf::from("nonexistent_path");
        assert_eq!(Identity::load_identity(&path).unwrap(), None);
    }

    #[test]
    fn test_load_identity_existing() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let private_key = x25519::PrivateKey::generate(&mut OsRng);
        temp_file.write_all(&private_key.to_bytes()).unwrap();
        let loaded_identity = Identity::load_identity(&temp_file.path().to_path_buf());
        match loaded_identity {
            Ok(Some(Identity::FromConfig(IdentityFromConfig { key: config, .. }))) => {
                assert_eq!(config.private_key(), private_key);
            },
            _ => panic!("Expected identity to be loaded from file"),
        }
    }

    #[test]
    fn test_auto_generated_identities_persist() {
        let (storage_config, temp_dir) = setup_storage_config_with_temp_dir();
        let key_path = temp_dir.join(IDENTITY_KEY_FILE);

        let network_config = NetworkConfig::default();
        let auto_generated_key = match network_config.identity.clone() {
            Identity::FromConfig(IdentityFromConfig {
                source: IdentitySource::AutoGenerated,
                key,
                ..
            }) => key,
            _ => panic!("Expected auto-generated key"),
        };

        let mut node_config = NodeConfig {
            storage: storage_config,
            full_node_networks: vec![network_config],
            ..Default::default()
        };

        assert!(
            !key_path.exists(),
            "The key file should not exist before optimizing the config"
        );

        optimize_public_network_config(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(),
            NodeType::PublicFullnode,
            Some(ChainId::testnet()),
        )
        .unwrap();

        let loaded_identity = Identity::load_identity(&key_path).unwrap();
        if let Some(Identity::FromConfig(IdentityFromConfig {
            key: loaded_key, ..
        })) = loaded_identity
        {
            assert_eq!(loaded_key.private_key(), auto_generated_key.private_key());
        } else {
            panic!("Expected identity to be loaded from file");
        }
    }
}
