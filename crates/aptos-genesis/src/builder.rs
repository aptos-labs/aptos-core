// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::ValidatorConfiguration,
    keys::{generate_key_objects, PrivateIdentity, PublicIdentity},
    GenesisInfo,
};
use anyhow::ensure;
use aptos_config::{
    config::{
        DiscoveryMethod, Identity, IdentityBlob, InitialSafetyRulesConfig, NetworkConfig,
        NodeConfig, OnDiskStorageConfig, OverrideNodeConfig, PeerRole, PersistableConfig, RoleType,
        SafetyRulesService, SecureBackend, WaypointConfig,
    },
    generator::build_seed_for_network,
    keys::ConfigKey,
    network_id::NetworkId,
};
use aptos_crypto::{
    bls12381,
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    PrivateKey,
};
use aptos_framework::ReleaseBundle;
use aptos_keygen::KeyGen;
use aptos_logger::prelude::*;
use aptos_types::{
    chain_id::ChainId,
    jwks::patch::IssuerJWK,
    keyless::Groth16VerificationKey,
    on_chain_config::{
        Features, GasScheduleV2, OnChainConsensusConfig, OnChainExecutionConfig,
        OnChainJWKConsensusConfig, OnChainRandomnessConfig,
    },
    transaction::Transaction,
    waypoint::Waypoint,
};
use aptos_vm_genesis::default_gas_schedule;
use rand::Rng;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    convert::{TryFrom, TryInto},
    fs::File,
    io::{Read, Write},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    sync::Arc,
};

const VALIDATOR_IDENTITY: &str = "validator-identity.yaml";
const VFN_IDENTITY: &str = "vfn-identity.yaml";
const PRIVATE_IDENTITY: &str = "private-identity.yaml";
const PUBLIC_IDENTITY: &str = "public-identity.yaml";
const CONFIG_FILE: &str = "node.yaml";
const GENESIS_BLOB: &str = "genesis.blob";

/// Configuration to run a local validator node
#[derive(Debug, Clone)]
pub struct ValidatorNodeConfig {
    pub name: String,
    pub index: usize,
    pub config: OverrideNodeConfig,
    pub dir: PathBuf,
    pub account_private_key: Option<ConfigKey<Ed25519PrivateKey>>,
    pub genesis_stake_amount: u64,
    pub commission_percentage: u64,
}

impl ValidatorNodeConfig {
    /// Create a new validator and initialize keys appropriately
    pub fn new(
        name: String,
        index: usize,
        base_dir: &Path,
        mut config: OverrideNodeConfig,
        genesis_stake_amount: u64,
        commission_percentage: u64,
    ) -> anyhow::Result<ValidatorNodeConfig> {
        // Create the data dir and set it appropriately
        let dir = base_dir.join(&name);
        std::fs::create_dir_all(dir.as_path())?;
        config.override_config_mut().set_data_dir(dir.clone());

        Ok(ValidatorNodeConfig {
            name,
            index,
            config,
            dir,
            account_private_key: None,
            genesis_stake_amount,
            commission_percentage,
        })
    }

    /// Initializes keys and identities for a validator config
    /// TODO: Put this all in storage rather than files?
    fn init_keys(&mut self, seed: Option<[u8; 32]>) -> anyhow::Result<()> {
        let (validator_identity, _, _, _) = self.get_key_objects(seed)?;
        self.account_private_key = validator_identity.account_private_key.map(ConfigKey::new);

        // Init network identity
        let config = self.config.override_config_mut();
        let validator_network = config.validator_network.as_mut().unwrap();
        let validator_identity_file = self.dir.join(VALIDATOR_IDENTITY);
        validator_network.identity = Identity::from_file(validator_identity_file);

        Ok(())
    }

    /// Allows for on disk caching of already generated keys
    pub fn get_key_objects(
        &self,
        seed: Option<[u8; 32]>,
    ) -> anyhow::Result<(IdentityBlob, IdentityBlob, PrivateIdentity, PublicIdentity)> {
        let dir = &self.dir;
        let val_identity_file = dir.join(VALIDATOR_IDENTITY);
        let vfn_identity_file = dir.join(VFN_IDENTITY);
        let private_identity_file = dir.join(PRIVATE_IDENTITY);
        let public_identity_file = dir.join(PUBLIC_IDENTITY);

        // If they all already exist, use them, otherwise generate new ones and overwrite
        if val_identity_file.exists()
            && vfn_identity_file.exists()
            && private_identity_file.exists()
            && public_identity_file.exists()
        {
            Ok((
                read_yaml(val_identity_file.as_path())?,
                read_yaml(vfn_identity_file.as_path())?,
                read_yaml(private_identity_file.as_path())?,
                read_yaml(public_identity_file.as_path())?,
            ))
        } else {
            let mut key_generator = if let Some(seed) = seed {
                KeyGen::from_seed(seed)
            } else {
                KeyGen::from_os_rng()
            };

            let (validator_identity, vfn_identity, private_identity, public_identity) =
                generate_key_objects(&mut key_generator)?;

            // Write identities in files
            write_yaml(val_identity_file.as_path(), &validator_identity)?;
            write_yaml(vfn_identity_file.as_path(), &vfn_identity)?;
            write_yaml(private_identity_file.as_path(), &private_identity)?;
            write_yaml(public_identity_file.as_path(), &public_identity)?;
            Ok((
                validator_identity,
                vfn_identity,
                private_identity,
                public_identity,
            ))
        }
    }

    fn insert_genesis(&mut self, genesis: &Transaction) {
        let config = self.config.override_config_mut();
        config.execution.genesis = Some(genesis.clone());
        config.execution.genesis_file_location = self.dir.join(GENESIS_BLOB)
    }

    fn insert_waypoint(&mut self, waypoint: &Waypoint) {
        let config = self.config.override_config_mut();
        let waypoint_config = WaypointConfig::FromConfig(*waypoint);

        // Init safety rules
        let validator_identity_file = self.dir.join(VALIDATOR_IDENTITY);
        config.consensus.safety_rules.initial_safety_rules_config =
            InitialSafetyRulesConfig::from_file(
                validator_identity_file,
                vec![],
                waypoint_config.clone(),
            );
        config.base.waypoint = waypoint_config;
    }

    fn save_config(&mut self) -> anyhow::Result<()> {
        // Save the execution config to disk along with the full config.
        self.config
            .override_config_mut()
            .save_to_path(self.dir.join(CONFIG_FILE))?;

        // Overwrite the full config with the override config
        self.config
            .save_config(self.dir.join(CONFIG_FILE))
            .map_err(Into::into)
    }
}

impl TryFrom<&ValidatorNodeConfig> for ValidatorConfiguration {
    type Error = anyhow::Error;

    fn try_from(config: &ValidatorNodeConfig) -> Result<Self, Self::Error> {
        let (_, _, private_identity, _) = config.get_key_objects(None)?;
        let validator_host = (&config
            .config
            .override_config()
            .validator_network
            .as_ref()
            .unwrap()
            .listen_address)
            .try_into()?;
        let full_node_host = Some(
            (&config
                .config
                .override_config()
                .full_node_networks
                .iter()
                .find(|network| network.network_id == NetworkId::Public)
                .unwrap()
                .listen_address)
                .try_into()?,
        );
        Ok(ValidatorConfiguration {
            owner_account_address: private_identity.account_address.into(),
            owner_account_public_key: private_identity.account_private_key.public_key(),
            operator_account_address: private_identity.account_address.into(),
            operator_account_public_key: private_identity.account_private_key.public_key(),
            voter_account_address: private_identity.account_address.into(),
            voter_account_public_key: private_identity.account_private_key.public_key(),
            consensus_public_key: Some(private_identity.consensus_private_key.public_key()),
            proof_of_possession: Some(bls12381::ProofOfPossession::create(
                &private_identity.consensus_private_key,
            )),
            validator_network_public_key: Some(
                private_identity.validator_network_private_key.public_key(),
            ),
            validator_host: Some(validator_host),
            full_node_network_public_key: Some(
                private_identity.full_node_network_private_key.public_key(),
            ),
            full_node_host,
            stake_amount: config.genesis_stake_amount,
            commission_percentage: config.commission_percentage,
            // Default to joining the genesis validator set.
            join_during_genesis: true,
        })
    }
}

pub struct FullnodeNodeConfig {
    pub name: String,
    pub config: OverrideNodeConfig,
    pub dir: PathBuf,
}

impl FullnodeNodeConfig {
    pub fn public_fullnode(
        name: String,
        config_dir: &Path,
        config: OverrideNodeConfig,
        waypoint: &Waypoint,
        genesis: &Transaction,
    ) -> anyhow::Result<Self> {
        let mut fullnode_config = Self::new(name, config_dir, config)?;

        fullnode_config.insert_waypoint(waypoint);
        fullnode_config.insert_genesis(genesis)?;
        fullnode_config.set_identity()?;
        fullnode_config.randomize_ports();
        fullnode_config.save_config()?;

        Ok(fullnode_config)
    }

    pub fn validator_fullnode(
        name: String,
        config_dir: &Path,
        fullnode_config: OverrideNodeConfig,
        validator_config: &NodeConfig,
        waypoint: &Waypoint,
        genesis: &Transaction,
        public_network: &NetworkConfig,
    ) -> anyhow::Result<Self> {
        let mut fullnode_config = Self::new(name, config_dir, fullnode_config)?;

        fullnode_config.insert_waypoint(waypoint);
        fullnode_config.insert_genesis(genesis)?;
        fullnode_config.randomize_ports();
        fullnode_config.attach_to_validator(public_network, validator_config)?;
        fullnode_config.save_config()?;

        Ok(fullnode_config)
    }

    fn new(
        name: String,
        config_dir: &Path,
        mut config: OverrideNodeConfig,
    ) -> anyhow::Result<Self> {
        let inner = config.override_config_mut();

        ensure!(
            matches!(inner.base.role, RoleType::FullNode),
            "config must be a FullNode config"
        );

        let dir = config_dir.join(&name);
        std::fs::create_dir_all(&dir)?;

        inner.set_data_dir(dir.clone());

        Ok(Self { name, config, dir })
    }

    fn insert_waypoint(&mut self, waypoint: &Waypoint) {
        let config = self.config.override_config_mut();
        config.base.waypoint = WaypointConfig::FromConfig(*waypoint);
    }

    fn insert_genesis(&mut self, genesis: &Transaction) -> anyhow::Result<()> {
        // Save genesis file in this validator's config dir
        let genesis_file_location = self.dir.join("genesis.blob");
        File::create(&genesis_file_location)?.write_all(&bcs::to_bytes(&genesis)?)?;

        let config = self.config.override_config_mut();
        config.execution.genesis = Some(genesis.clone());
        config.execution.genesis_file_location = genesis_file_location;

        Ok(())
    }

    fn randomize_ports(&mut self) {
        let config = self.config.override_config_mut();
        config.randomize_ports();
    }

    /// Sets identity for a public full node.  Should only be run on a public full node
    fn set_identity(&mut self) -> anyhow::Result<()> {
        let config = self.config.override_config_mut();

        if config
            .full_node_networks
            .iter()
            .any(|config| config.network_id == NetworkId::Vfn)
        {
            panic!("Shouldn't call set_identity on a Validator full node");
        }

        let public_network = config
            .full_node_networks
            .iter_mut()
            .find(|config| config.network_id == NetworkId::Public)
            .unwrap();

        set_identity_for_network(public_network)?;
        Ok(())
    }

    /// Attaches a Full node to a validator full node
    fn attach_to_validator(
        &mut self,
        public_network: &NetworkConfig,
        validator_config: &NodeConfig,
    ) -> anyhow::Result<()> {
        ensure!(
            matches!(validator_config.base.role, RoleType::Validator),
            "Validator config must be a Validator config"
        );

        let config = self.config.override_config_mut();

        let fullnode_public_network = config
            .full_node_networks
            .iter_mut()
            .find(|config| config.network_id == NetworkId::Public)
            .expect("VFN should have a public network");
        fullnode_public_network.identity = public_network.identity.clone();
        fullnode_public_network.listen_address = public_network.listen_address.clone();

        // Grab the validator's vfn network information and configure it as a seed for the VFN's
        // vfn network
        let validators_vfn_network = validator_config
            .full_node_networks
            .iter()
            .find(|config| config.network_id.is_vfn_network())
            .expect("Validator should have vfn network");

        let fullnode_vfn_network = config
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

    fn save_config(&mut self) -> anyhow::Result<()> {
        self.config
            .save_config(self.dir.join(CONFIG_FILE))
            .map_err(Into::into)
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

const ONE_DAY: u64 = 86400;

#[derive(Clone)]
pub struct GenesisConfiguration {
    pub allow_new_validators: bool,
    pub epoch_duration_secs: u64,
    pub is_test: bool,
    pub min_stake: u64,
    pub max_stake: u64,
    pub min_voting_threshold: u128,
    pub recurring_lockup_duration_secs: u64,
    pub required_proposer_stake: u64,
    pub rewards_apy_percentage: u64,
    pub voting_duration_secs: u64,
    pub voting_power_increase_limit: u64,
    pub employee_vesting_start: Option<u64>,
    pub employee_vesting_period_duration: Option<u64>,
    pub consensus_config: OnChainConsensusConfig,
    pub execution_config: OnChainExecutionConfig,
    pub gas_schedule: GasScheduleV2,
    pub initial_features_override: Option<Features>,
    pub randomness_config_override: Option<OnChainRandomnessConfig>,
    pub jwk_consensus_config_override: Option<OnChainJWKConsensusConfig>,
    pub initial_jwks: Vec<IssuerJWK>,
    pub keyless_groth16_vk: Option<Groth16VerificationKey>,
}

pub type InitConfigFn = Arc<dyn Fn(usize, &mut NodeConfig, &mut NodeConfig) + Send + Sync>;
pub type InitGenesisStakeFn = Arc<dyn Fn(usize, &mut u64) + Send + Sync>;
pub type InitGenesisConfigFn = Arc<dyn Fn(&mut GenesisConfiguration) + Send + Sync>;

/// Builder that builds a network of validator nodes that can run locally
#[derive(Clone)]
pub struct Builder {
    config_dir: PathBuf,
    framework: ReleaseBundle,
    num_validators: NonZeroUsize,
    randomize_first_validator_ports: bool,
    init_config: Option<InitConfigFn>,
    init_genesis_stake: Option<InitGenesisStakeFn>,
    init_genesis_config: Option<InitGenesisConfigFn>,
}

impl Builder {
    pub fn new(config_dir: &Path, framework: ReleaseBundle) -> anyhow::Result<Self> {
        let config_dir: PathBuf = config_dir.into();
        let config_dir = config_dir.canonicalize()?;

        Ok(Self {
            config_dir,
            framework,
            num_validators: NonZeroUsize::new(1).unwrap(),
            randomize_first_validator_ports: true,
            init_config: None,
            init_genesis_stake: None,
            init_genesis_config: None,
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

    pub fn with_init_config(mut self, init_config: Option<InitConfigFn>) -> Self {
        self.init_config = init_config;
        self
    }

    pub fn with_init_genesis_stake(
        mut self,
        init_genesis_stake: Option<InitGenesisStakeFn>,
    ) -> Self {
        self.init_genesis_stake = init_genesis_stake;
        self
    }

    pub fn with_init_genesis_config(
        mut self,
        init_genesis_config: Option<InitGenesisConfigFn>,
    ) -> Self {
        self.init_genesis_config = init_genesis_config;
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
        // We use this print statement to allow debugging of local deployments
        info!(
            "Building genesis with {:?} validators. Directory of output: {:?}",
            self.num_validators.get(),
            self.config_dir
        );

        // Generate root key
        let mut keygen = KeyGen::from_seed(rng.r#gen());
        let root_key = keygen.generate_ed25519_private_key();

        // Generate validator configs
        let template = NodeConfig::get_default_validator_config();

        let mut validators: Vec<ValidatorNodeConfig> = (0..self.num_validators.get())
            .map(|i| self.generate_validator_config(i, &mut rng, &template))
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
        template: &NodeConfig,
    ) -> anyhow::Result<ValidatorNodeConfig>
    where
        R: rand::RngCore + rand::CryptoRng,
    {
        let name = index.to_string();

        let mut override_config = template.clone();
        let mut base_config = NodeConfig::default();
        if let Some(init_config) = &self.init_config {
            (init_config)(index, &mut override_config, &mut base_config);
        }

        let mut validator = ValidatorNodeConfig::new(
            name,
            index,
            self.config_dir.as_path(),
            OverrideNodeConfig::new(override_config, base_config),
            // Default value. Can be overriden by init_genesis_stake
            10,
            // Default to 0% commission for local node building.
            0,
        )?;

        validator.init_keys(Some(rng.r#gen()))?;

        // By default, we don't start with VFNs, so ensure that the REST port is open
        let vfn_identity_path = validator.dir.join(VFN_IDENTITY);

        let config = &mut validator.config.override_config_mut();
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

        // Use a file based storage backend for safety rules
        let mut storage = OnDiskStorageConfig::default();
        storage.set_data_dir(validator.dir.clone());
        config.consensus.safety_rules.backend = SecureBackend::OnDiskStorage(storage);

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

        if let Some(init_genesis_stake) = &self.init_genesis_stake {
            for validator in validators.iter_mut() {
                (init_genesis_stake)(validator.index, &mut validator.genesis_stake_amount);
            }
        }
        for validator in validators.iter() {
            configs.push(validator.try_into()?);
        }

        let mut genesis_config = GenesisConfiguration {
            allow_new_validators: false,
            epoch_duration_secs: ONE_DAY,
            is_test: true,
            min_stake: 0,
            min_voting_threshold: 0,
            max_stake: u64::MAX,
            recurring_lockup_duration_secs: ONE_DAY,
            required_proposer_stake: 0,
            rewards_apy_percentage: 10,
            voting_duration_secs: ONE_DAY / 24,
            voting_power_increase_limit: 50,
            employee_vesting_start: None,
            employee_vesting_period_duration: None,
            consensus_config: OnChainConsensusConfig::default_for_genesis(),
            execution_config: OnChainExecutionConfig::default_for_genesis(),
            gas_schedule: default_gas_schedule(),
            initial_features_override: None,
            randomness_config_override: None,
            jwk_consensus_config_override: None,
            initial_jwks: vec![],
            keyless_groth16_vk: None,
        };
        if let Some(init_genesis_config) = &self.init_genesis_config {
            (init_genesis_config)(&mut genesis_config);
        }

        // Build genesis & waypoint
        let mut genesis_info = GenesisInfo::new(
            ChainId::test(),
            root_key,
            configs,
            self.framework.clone(),
            &genesis_config,
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
