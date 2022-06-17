// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::ValidatorConfiguration,
    keys::{generate_key_objects, PrivateIdentity},
    GenesisInfo,
};
use anyhow::ensure;
use aptos_config::{
    config::{
        DiscoveryMethod, Identity, IdentityBlob, InitialSafetyRulesConfig, NetworkConfig,
        NodeConfig, PeerRole, RoleType, SafetyRulesService, WaypointConfig,
    },
    generator::build_seed_for_network,
    network_id::NetworkId,
};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    PrivateKey,
};
use aptos_keygen::KeyGen;
use aptos_types::{chain_id::ChainId, transaction::Transaction, waypoint::Waypoint};
use rand::Rng;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    convert::{TryFrom, TryInto},
    fs::File,
    io::{Read, Write},
    num::NonZeroUsize,
    path::{Path, PathBuf},
};

const VALIDATOR_IDENTITY: &str = "validator-identity.yaml";
const VFN_IDENTITY: &str = "vfn-identity.yaml";
const PRIVATE_IDENTITY: &str = "private-identity.yaml";
const CONFIG_FILE: &str = "node.yaml";
const GENESIS_BLOB: &str = "genesis.blob";

/// Configuration to run a local validator node
#[derive(Debug, Clone)]
pub struct ValidatorNodeConfig {
    pub name: String,
    pub config: NodeConfig,
    pub dir: PathBuf,
}

impl ValidatorNodeConfig {
    /// Create a new validator and initialize keys appropriately
    pub fn new(
        name: String,
        base_dir: &Path,
        mut config: NodeConfig,
    ) -> anyhow::Result<ValidatorNodeConfig> {
        // Create the data dir and set it appropriately
        let dir = base_dir.join(&name);
        std::fs::create_dir_all(dir.as_path())?;
        config.set_data_dir(dir.clone());

        Ok(ValidatorNodeConfig { name, config, dir })
    }

    /// Initializes keys and identities for a validator config
    /// TODO: Put this all in storage rather than files?
    fn init_keys(&mut self, seed: Option<[u8; 32]>) -> anyhow::Result<()> {
        self.get_key_objects(seed)?;

        // Init network identity
        let validator_network = self.config.validator_network.as_mut().unwrap();
        let validator_identity_file = self.dir.join(VALIDATOR_IDENTITY);
        validator_network.identity = Identity::from_file(validator_identity_file);

        Ok(())
    }

    /// Allows for on disk caching of already generated keys
    pub fn get_key_objects(
        &self,
        seed: Option<[u8; 32]>,
    ) -> anyhow::Result<(IdentityBlob, IdentityBlob, PrivateIdentity)> {
        let dir = &self.dir;
        let val_identity_file = dir.join(VALIDATOR_IDENTITY);
        let vfn_identity_file = dir.join(VFN_IDENTITY);
        let private_identity_file = dir.join(PRIVATE_IDENTITY);

        // If they all already exist, use them, otherwise generate new ones and overwrite
        if val_identity_file.exists()
            && vfn_identity_file.exists()
            && private_identity_file.exists()
        {
            Ok((
                read_yaml(val_identity_file.as_path())?,
                read_yaml(vfn_identity_file.as_path())?,
                read_yaml(private_identity_file.as_path())?,
            ))
        } else {
            let mut key_generator = if let Some(seed) = seed {
                KeyGen::from_seed(seed)
            } else {
                KeyGen::from_os_rng()
            };

            let (validator_identity, vfn_identity, private_identity) =
                generate_key_objects(&mut key_generator)?;

            // Write identities in files
            write_yaml(val_identity_file.as_path(), &validator_identity)?;
            write_yaml(vfn_identity_file.as_path(), &vfn_identity)?;
            write_yaml(private_identity_file.as_path(), &private_identity)?;
            Ok((validator_identity, vfn_identity, private_identity))
        }
    }

    fn insert_genesis(&mut self, genesis: &Transaction) {
        self.config.execution.genesis = Some(genesis.clone());
        self.config.execution.genesis_file_location = self.dir.join(GENESIS_BLOB)
    }

    fn insert_waypoint(&mut self, waypoint: &Waypoint) {
        let waypoint_config = WaypointConfig::FromConfig(*waypoint);

        // Init safety rules
        let validator_identity_file = self.dir.join(VALIDATOR_IDENTITY);
        self.config
            .consensus
            .safety_rules
            .initial_safety_rules_config =
            InitialSafetyRulesConfig::from_file(validator_identity_file, waypoint_config.clone());
        self.config.base.waypoint = waypoint_config;
    }

    fn save_config(&mut self) -> anyhow::Result<()> {
        Ok(self.config.save(self.dir.join(CONFIG_FILE))?)
    }
}

impl TryFrom<&ValidatorNodeConfig> for ValidatorConfiguration {
    type Error = anyhow::Error;

    fn try_from(config: &ValidatorNodeConfig) -> Result<Self, Self::Error> {
        let (_, _, private_identity) = config.get_key_objects(None)?;
        let validator_host = (&config
            .config
            .validator_network
            .as_ref()
            .unwrap()
            .listen_address)
            .try_into()?;
        let full_node_host = Some(
            (&config
                .config
                .full_node_networks
                .iter()
                .find(|network| network.network_id == NetworkId::Public)
                .unwrap()
                .listen_address)
                .try_into()?,
        );
        Ok(ValidatorConfiguration {
            account_address: private_identity.account_address,
            consensus_public_key: private_identity.consensus_private_key.public_key(),
            account_public_key: private_identity.account_private_key.public_key(),
            validator_network_public_key: private_identity
                .validator_network_private_key
                .public_key(),
            validator_host,
            full_node_network_public_key: Some(
                private_identity.full_node_network_private_key.public_key(),
            ),
            full_node_host,
            stake_amount: 1,
        })
    }
}

pub struct FullnodeNodeConfig {
    pub name: String,
    pub config: NodeConfig,
    pub dir: PathBuf,
}

impl FullnodeNodeConfig {
    pub fn public_fullnode(
        name: String,
        config_dir: &Path,
        config: NodeConfig,
        waypoint: &Waypoint,
        genesis: &Transaction,
    ) -> anyhow::Result<Self> {
        let mut fullnode_config = Self::new(name, config_dir, config)?;

        fullnode_config.insert_waypoint(waypoint);
        fullnode_config.insert_genesis(genesis)?;
        fullnode_config.set_identity()?;
        fullnode_config.config.randomize_ports();
        fullnode_config.save_config()?;

        Ok(fullnode_config)
    }

    pub fn validator_fullnode(
        name: String,
        config_dir: &Path,
        fullnode_config: NodeConfig,
        validator_config: &mut NodeConfig,
        waypoint: &Waypoint,
        genesis: &Transaction,
    ) -> anyhow::Result<Self> {
        let mut fullnode_config = Self::new(name, config_dir, fullnode_config)?;

        fullnode_config.insert_waypoint(waypoint);
        fullnode_config.insert_genesis(genesis)?;
        fullnode_config.config.randomize_ports();
        fullnode_config.attach_to_validator(validator_config)?;
        fullnode_config.save_config()?;

        Ok(fullnode_config)
    }

    fn new(name: String, config_dir: &Path, mut config: NodeConfig) -> anyhow::Result<Self> {
        ensure!(
            matches!(config.base.role, RoleType::FullNode),
            "config must be a FullNode config"
        );

        let dir = config_dir.join(&name);
        std::fs::create_dir_all(&dir)?;

        config.set_data_dir(dir.clone());

        Ok(Self { name, config, dir })
    }

    fn insert_waypoint(&mut self, waypoint: &Waypoint) {
        self.config.base.waypoint = WaypointConfig::FromConfig(*waypoint);
    }

    fn insert_genesis(&mut self, genesis: &Transaction) -> anyhow::Result<()> {
        // Save genesis file in this validator's config dir
        let genesis_file_location = self.dir.join("genesis.blob");
        File::create(&genesis_file_location)?.write_all(&bcs::to_bytes(&genesis)?)?;

        self.config.execution.genesis = Some(genesis.clone());
        self.config.execution.genesis_file_location = genesis_file_location;

        Ok(())
    }

    /// Sets identity for a public full node.  Should only be run on a public full node
    fn set_identity(&mut self) -> anyhow::Result<()> {
        if self
            .config
            .full_node_networks
            .iter()
            .any(|config| config.network_id == NetworkId::Vfn)
        {
            panic!("Shouldn't call set_identity on a Validator full node");
        }

        let public_network = self
            .config
            .full_node_networks
            .iter_mut()
            .find(|config| config.network_id == NetworkId::Public)
            .unwrap();

        set_identity_for_network(public_network)?;
        Ok(())
    }

    /// Attaches a Full node to a validator full node
    fn attach_to_validator(&mut self, validator_config: &mut NodeConfig) -> anyhow::Result<()> {
        ensure!(
            matches!(validator_config.base.role, RoleType::Validator),
            "Validator config must be a Validator config"
        );

        // Grab the public network config from the validator and insert it into the VFN's config
        // The validator's public network identity is the same as the VFN's public network identity
        // We remove it from the validator so the VFN can hold it
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

        // Set the identity for the VFN port
        set_identity_for_network(fullnode_vfn_network)?;

        Ok(())
    }

    pub fn config_path(&self) -> PathBuf {
        self.dir.join("node.yaml")
    }

    fn save_config(&mut self) -> anyhow::Result<()> {
        self.config.save(self.config_path()).map_err(Into::into)
    }
}

fn set_identity_for_network(network: &mut NetworkConfig) -> anyhow::Result<()> {
    if let Identity::None = network.identity {
        let mut keygen = KeyGen::from_os_rng();
        let key = keygen.generate_x25519_private_key()?;
        let peer_id = aptos_types::account_address::from_identity_public_key(key.public_key());
        network.identity = Identity::from_config(key, peer_id);
    }
    Ok(())
}

fn read_yaml<T: DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let mut string = String::new();
    File::open(path)?.read_to_string(&mut string)?;
    Ok(serde_yaml::from_str(&string)?)
}

fn write_yaml<T: Serialize>(path: &Path, object: &T) -> anyhow::Result<()> {
    File::create(path)?.write_all(serde_yaml::to_string(object)?.as_bytes())?;
    Ok(())
}

/// Builder that builds a network of validator nodes that can run locally
#[derive(Clone)]
pub struct Builder {
    config_dir: PathBuf,
    move_modules: Vec<Vec<u8>>,
    num_validators: NonZeroUsize,
    randomize_first_validator_ports: bool,
    template: NodeConfig,
    min_price_per_gas_unit: u64,
}

impl Builder {
    pub fn new(config_dir: &Path, move_modules: Vec<Vec<u8>>) -> anyhow::Result<Self> {
        let config_dir: PathBuf = config_dir.into();
        let config_dir = config_dir.canonicalize()?;
        Ok(Self {
            config_dir,
            move_modules,
            num_validators: NonZeroUsize::new(1).unwrap(),
            randomize_first_validator_ports: true,
            template: NodeConfig::default_for_validator(),
            min_price_per_gas_unit: 1,
        })
    }

    pub fn with_randomize_first_validator_ports(mut self, value: bool) -> Self {
        self.randomize_first_validator_ports = value;
        self
    }

    pub fn with_num_validators(mut self, num_validators: NonZeroUsize) -> Self {
        self.num_validators = num_validators;
        self
    }

    pub fn with_template(mut self, template: NodeConfig) -> Self {
        self.template = template;
        self
    }

    pub fn with_min_price_per_gas_unit(mut self, min_price_per_gas_unit: u64) -> Self {
        self.min_price_per_gas_unit = min_price_per_gas_unit;
        self
    }

    /// Build all of the validators and save their configs
    pub fn build<R>(
        mut self,
        mut rng: R,
    ) -> anyhow::Result<(
        Ed25519PrivateKey,
        Transaction,
        Waypoint,
        Vec<ValidatorNodeConfig>,
    )>
    where
        R: rand::RngCore + rand::CryptoRng,
    {
        let mut keygen = KeyGen::from_seed(rng.gen());

        // Generate root key
        let root_key = keygen.generate_ed25519_private_key();

        // Generate validator configs
        let mut validators: Vec<ValidatorNodeConfig> = (0..self.num_validators.get())
            .map(|i| self.generate_validator_config(i, &mut rng))
            .collect::<anyhow::Result<Vec<ValidatorNodeConfig>>>()?;

        // Build genesis
        let (genesis, waypoint) = self.genesis_ceremony(&mut validators, root_key.public_key())?;

        // Save configs for validators so they can run
        for validator in validators.iter_mut() {
            validator.save_config()?;
        }

        Ok((root_key, genesis, waypoint, validators))
    }

    /// Generate a configuration for a single validator
    fn generate_validator_config<R>(
        &mut self,
        index: usize,
        mut rng: R,
    ) -> anyhow::Result<ValidatorNodeConfig>
    where
        R: rand::RngCore + rand::CryptoRng,
    {
        let name = index.to_string();

        let mut validator =
            ValidatorNodeConfig::new(name, self.config_dir.as_path(), self.template.clone())?;

        validator.init_keys(Some(rng.gen()))?;

        // By default, we don't start with VFNs, so ensure that the REST port is open
        let vfn_identity_path = validator.dir.join(VFN_IDENTITY);

        let config = &mut validator.config;
        let fullnode_network_listen_address =
            if let Some(template_fullnode_config) = config.full_node_networks.first() {
                template_fullnode_config.listen_address.clone()
            } else {
                aptos_config::utils::get_available_port_in_multiaddr(true)
            };

        let fullnode_network = NetworkConfig {
            listen_address: fullnode_network_listen_address,
            network_id: NetworkId::Public,
            max_outbound_connections: 0,
            discovery_method: DiscoveryMethod::Onchain,
            identity: Identity::from_file(vfn_identity_path.clone()),
            ..Default::default()
        };

        // VFN has the same credentials as the public full node identity
        let vfn_network = NetworkConfig {
            listen_address: aptos_config::utils::get_available_port_in_multiaddr(true),
            network_id: NetworkId::Vfn,
            max_outbound_connections: 0,
            identity: Identity::from_file(vfn_identity_path),
            ..Default::default()
        };

        config.full_node_networks = vec![fullnode_network, vfn_network];

        // Ensure safety rules runs in a thread
        config.consensus.safety_rules.service = SafetyRulesService::Thread;

        if index > 0 || self.randomize_first_validator_ports {
            config.randomize_ports();
        }

        Ok(validator)
    }

    /// Do the genesis ceremony and copy waypoint and genesis to each node
    fn genesis_ceremony(
        &mut self,
        validators: &mut Vec<ValidatorNodeConfig>,
        root_key: Ed25519PublicKey,
    ) -> anyhow::Result<(Transaction, Waypoint)> {
        let mut configs: Vec<ValidatorConfiguration> = Vec::new();

        for validator in validators.iter() {
            configs.push(validator.try_into()?);
        }

        // Build genesis & waypoint
        let mut genesis_info = GenesisInfo::new(
            ChainId::test(),
            root_key,
            configs,
            self.move_modules.clone(),
            self.min_price_per_gas_unit,
            false,
            0,
            u64::MAX,
            86400,    // 1 day
            31536000, // 1 year
            0,
            0,
        )?;
        let waypoint = genesis_info.generate_waypoint()?;
        let genesis = genesis_info.get_genesis();

        // Insert genesis and waypoint into validators
        // TODO: verify genesis?
        for validator in validators {
            validator.insert_waypoint(&waypoint);
            validator.insert_genesis(genesis);
        }

        Ok((genesis.clone(), waypoint))
    }
}
