// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{ensure, Result};
use diem_config::{
    config::{Identity, NodeConfig, PeerRole, RoleType, WaypointConfig},
    generator::build_seed_for_network,
    network_id::NetworkId,
};
use diem_crypto::{x25519, Uniform};
use diem_types::{transaction::Transaction, waypoint::Waypoint};
use rand::rngs::OsRng;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

pub struct FullnodeConfig {
    pub name: String,
    pub config: NodeConfig,
    pub directory: PathBuf,
}

impl FullnodeConfig {
    pub fn public_fullnode(
        name: String,
        config_directory: &Path,
        config: NodeConfig,
        waypoint: &Waypoint,
        genesis: &Transaction,
    ) -> Result<Self> {
        let mut fullnode_config = Self::new(name, config_directory, config)?;

        fullnode_config.insert_waypoint(waypoint);
        fullnode_config.insert_genesis(genesis)?;
        fullnode_config.set_identity();
        fullnode_config.config.randomize_ports();
        fullnode_config.save_config()?;

        Ok(fullnode_config)
    }

    pub fn validator_fullnode(
        name: String,
        config_directory: &Path,
        fullnode_config: NodeConfig,
        validator_config: &mut NodeConfig,
        waypoint: &Waypoint,
        genesis: &Transaction,
    ) -> Result<Self> {
        let mut fullnode_config = Self::new(name, config_directory, fullnode_config)?;

        fullnode_config.insert_waypoint(waypoint);
        fullnode_config.insert_genesis(genesis)?;
        fullnode_config.config.randomize_ports();

        fullnode_config.attach_to_validator(validator_config)?;
        fullnode_config.save_config()?;

        Ok(fullnode_config)
    }

    fn new(name: String, config_directory: &Path, mut config: NodeConfig) -> Result<Self> {
        ensure!(
            matches!(config.base.role, RoleType::FullNode),
            "config must be a FullNode config"
        );

        let directory = config_directory.join(&name);
        std::fs::create_dir_all(&directory)?;

        config.set_data_dir(directory.clone());

        Ok(Self {
            name,
            config,
            directory,
        })
    }

    fn insert_waypoint(&mut self, waypoint: &Waypoint) {
        self.config.base.waypoint = WaypointConfig::FromConfig(*waypoint);
    }

    fn insert_genesis(&mut self, genesis: &Transaction) -> Result<()> {
        // Save genesis file in this validator's config directory
        let genesis_file_location = self.directory.join("genesis.blob");
        File::create(&genesis_file_location)?.write_all(&bcs::to_bytes(&genesis)?)?;

        self.config.execution.genesis = Some(genesis.clone());
        self.config.execution.genesis_file_location = genesis_file_location;

        Ok(())
    }

    fn set_identity(&mut self) {
        let mut network_config = self
            .config
            .full_node_networks
            .iter_mut()
            .find(|config| config.network_id == NetworkId::Public)
            .unwrap();

        if let Identity::None = network_config.identity {
            let key = x25519::PrivateKey::generate(&mut OsRng);
            let peer_id = diem_types::account_address::from_identity_public_key(key.public_key());
            network_config.identity = Identity::from_config(key, peer_id);
        }
    }

    fn attach_to_validator(&mut self, validator_config: &mut NodeConfig) -> Result<()> {
        ensure!(
            matches!(validator_config.base.role, RoleType::Validator),
            "Validator config must be a Validator config"
        );

        // Grab the public network config from the validator and insert it into the VFN's config
        let public_network = {
            let (i, _) = validator_config
                .full_node_networks
                .iter()
                .enumerate()
                .find(|(_i, config)| config.network_id == NetworkId::Public)
                .expect("Validator should have a public network");
            validator_config.full_node_networks.remove(i)
        };

        let fullnode_public_network = self
            .config
            .full_node_networks
            .iter_mut()
            .find(|config| config.network_id == NetworkId::Public)
            .expect("VFN should have a public network");
        fullnode_public_network.identity = public_network.identity;
        fullnode_public_network.listen_address = public_network.listen_address;

        // Grab the validator's vfn network information and configure it as a seed for the VFN's
        // vfn network
        let validators_vfn_network = validator_config
            .full_node_networks
            .iter()
            .find(|config| config.network_id.is_vfn_network())
            .expect("Validator should have vfn network");

        let fullnode_vfn_network = self
            .config
            .full_node_networks
            .iter_mut()
            .find(|config| config.network_id.is_vfn_network())
            .expect("VFN should have a vfn network");
        fullnode_vfn_network.seeds =
            build_seed_for_network(validators_vfn_network, PeerRole::Validator);

        if let Identity::None = fullnode_vfn_network.identity {
            let key = x25519::PrivateKey::generate(&mut OsRng);
            let peer_id = diem_types::account_address::from_identity_public_key(key.public_key());
            fullnode_vfn_network.identity = Identity::from_config(key, peer_id);
        }

        Ok(())
    }

    pub fn config_path(&self) -> PathBuf {
        self.directory.join("node.yaml")
    }

    fn save_config(&mut self) -> Result<()> {
        self.config.save(self.config_path()).map_err(Into::into)
    }
}
