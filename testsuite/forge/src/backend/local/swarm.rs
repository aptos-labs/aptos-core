// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ChainInfo, FullNode, HealthCheckError, LocalNode, LocalVersion, Node, Swarm, SwarmChaos,
    SwarmExt, Validator, Version,
};
use anyhow::{anyhow, bail, Result};
use aptos_config::{
    config::{NetworkConfig, NodeConfig, OverrideNodeConfig, PersistableConfig},
    keys::ConfigKey,
    network_id::NetworkId,
};
use aptos_framework::ReleaseBundle;
use aptos_genesis::builder::{
    FullnodeNodeConfig, InitConfigFn, InitGenesisConfigFn, InitGenesisStakeFn,
};
use aptos_infallible::Mutex;
use aptos_logger::{info, warn};
use aptos_sdk::{
    crypto::{ed25519::Ed25519PrivateKey, encoding_type::EncodingType},
    types::{
        chain_id::ChainId, transaction::Transaction, waypoint::Waypoint, AccountKey, LocalAccount,
        PeerId,
    },
};
use prometheus_http_query::response::{PromqlResult, Sample};
use std::{
    collections::HashMap,
    fs,
    fs::File,
    io::Write,
    mem,
    num::NonZeroUsize,
    ops,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use tempfile::TempDir;

#[derive(Debug)]
pub enum SwarmDirectory {
    Persistent(PathBuf),
    Temporary(TempDir),
}

impl SwarmDirectory {
    pub fn persist(&mut self) {
        match self {
            SwarmDirectory::Persistent(_) => {},
            SwarmDirectory::Temporary(_) => {
                let mut temp = SwarmDirectory::Persistent(PathBuf::new());
                mem::swap(self, &mut temp);
                let _ = mem::replace(self, temp.into_persistent());
            },
        }
    }

    pub fn into_persistent(self) -> Self {
        match self {
            SwarmDirectory::Temporary(tempdir) => SwarmDirectory::Persistent(tempdir.into_path()),
            SwarmDirectory::Persistent(dir) => SwarmDirectory::Persistent(dir),
        }
    }
}

impl ops::Deref for SwarmDirectory {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        match self {
            SwarmDirectory::Persistent(dir) => dir.deref(),
            SwarmDirectory::Temporary(dir) => dir.path(),
        }
    }
}

impl AsRef<Path> for SwarmDirectory {
    fn as_ref(&self) -> &Path {
        match self {
            SwarmDirectory::Persistent(dir) => dir.as_ref(),
            SwarmDirectory::Temporary(dir) => dir.as_ref(),
        }
    }
}

#[derive(Debug)]
pub struct LocalSwarm {
    node_name_counter: usize,
    genesis: Transaction,
    genesis_waypoint: Waypoint,
    versions: Arc<HashMap<Version, LocalVersion>>,
    validators: HashMap<PeerId, LocalNode>,
    fullnodes: HashMap<PeerId, LocalNode>,
    public_networks: HashMap<PeerId, NetworkConfig>,
    dir: SwarmDirectory,
    root_account: Arc<LocalAccount>,
    chain_id: ChainId,
    root_key: ConfigKey<Ed25519PrivateKey>,

    launched: bool,
    #[allow(dead_code)]
    guard: ActiveNodesGuard,
}

impl LocalSwarm {
    pub fn build<R>(
        rng: R,
        number_of_validators: NonZeroUsize,
        versions: Arc<HashMap<Version, LocalVersion>>,
        initial_version: Option<Version>,
        init_config: Option<InitConfigFn>,
        init_genesis_stake: Option<InitGenesisStakeFn>,
        init_genesis_config: Option<InitGenesisConfigFn>,
        dir: Option<PathBuf>,
        genesis_framework: Option<ReleaseBundle>,
        guard: ActiveNodesGuard,
    ) -> Result<LocalSwarm>
    where
        R: ::rand::RngCore + ::rand::CryptoRng,
    {
        info!("Building a new swarm");
        let dir_actual = if let Some(dir_) = dir {
            if dir_.exists() {
                fs::remove_dir_all(&dir_)?;
            }
            fs::create_dir_all(&dir_)?;
            SwarmDirectory::Persistent(dir_)
        } else {
            SwarmDirectory::Temporary(TempDir::new()?)
        };

        let (root_key, genesis, genesis_waypoint, validators) =
            aptos_genesis::builder::Builder::new(
                &dir_actual,
                genesis_framework
                    .unwrap_or_else(|| aptos_cached_packages::head_release_bundle().clone()),
            )?
            .with_num_validators(number_of_validators)
            .with_init_config(Some(Arc::new(move |index, config, base| {
                // for local tests, turn off parallel execution:
                config.execution.concurrency_level = 1;

                // Single node orders blocks too fast which would trigger backpressure and stall for 1 sec
                // which cause flakiness in tests.
                if number_of_validators.get() == 1 {
                    // this delays empty block by (30-1) * 30ms
                    config.consensus.quorum_store_poll_time_ms = 900;
                    config
                        .state_sync
                        .state_sync_driver
                        .enable_auto_bootstrapping = true;
                    config
                        .state_sync
                        .state_sync_driver
                        .max_connection_deadline_secs = 1;
                }

                if let Some(init_config) = &init_config {
                    (init_config)(index, config, base);
                }
            })))
            .with_init_genesis_stake(init_genesis_stake)
            .with_init_genesis_config(init_genesis_config)
            .build(rng)?;

        // Get the initial version to start the nodes with, either the one provided or fallback to
        // using the latest version
        let initial_version_actual = initial_version.unwrap_or_else(|| {
            versions
                .iter()
                .max_by(|v1, v2| v1.0.cmp(v2.0))
                .unwrap()
                .0
                .clone()
        });
        let version = versions.get(&initial_version_actual).unwrap();

        let mut validators = validators
            .into_iter()
            .map(|v| {
                let node = LocalNode::new(
                    version.to_owned(),
                    v.name,
                    v.index,
                    v.dir,
                    v.account_private_key,
                )?;
                Ok((node.peer_id(), node))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        // After genesis, remove public network from validator and add to public_networks
        let public_networks = validators
            .values_mut()
            .map(|validator| {
                let mut validator_override_config =
                    OverrideNodeConfig::load_config(validator.config_path())?;
                let validator_config = validator_override_config.override_config_mut();

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
                validator_config.set_data_dir(validator.base_dir());
                *validator.config_mut() = validator_config.clone();
                // Since the validator's config has changed we need to save it
                validator_override_config.save_config(validator.config_path())?;

                Ok((validator.peer_id(), public_network))
            })
            .collect::<Result<HashMap<_, _>>>()?;

        // We print out the root key to make it easy for users to deploy a local faucet
        let encoded_root_key = EncodingType::Hex.encode_key("root_key", &root_key)?;
        info!(
            "The root (or mint) key for the swarm is: 0x{}",
            String::from_utf8_lossy(encoded_root_key.as_slice())
        );
        let root_key_path = dir_actual.as_ref().join("root_key");
        if let Ok(mut out) = File::create(root_key_path.clone()) {
            out.write_all(encoded_root_key.as_slice())?;
            info!("Wrote root (or mint) key to: {}", root_key_path.display());
        }
        let encoded_root_key = EncodingType::BCS.encode_key("root_key", &root_key)?;
        let root_key_path = dir_actual.as_ref().join("root_key.bin");
        if let Ok(mut out) = File::create(root_key_path.clone()) {
            out.write_all(encoded_root_key.as_slice())?;
            info!("Wrote root (or mint) key to: {}", root_key_path.display());
        }

        let root_key = ConfigKey::new(root_key);
        let root_account = LocalAccount::new(
            aptos_sdk::types::account_config::aptos_test_root_address(),
            AccountKey::from_private_key(root_key.private_key()),
            0,
        );
        let root_account = Arc::new(root_account);

        Ok(LocalSwarm {
            node_name_counter: validators.len(),
            genesis,
            genesis_waypoint,
            versions,
            validators,
            fullnodes: HashMap::new(),
            public_networks,
            dir: dir_actual,
            root_account,
            chain_id: ChainId::test(),
            root_key,
            launched: false,
            guard,
        })
    }

    pub async fn launch(&mut self) -> Result<()> {
        if self.launched {
            return Err(anyhow!("Swarm already launched"));
        }
        self.launched = true;

        // Start all the validators
        for validator in self.validators.values_mut() {
            validator.start()?;
        }

        self.wait_all_alive(Duration::from_secs(60)).await?;
        info!("Swarm launched successfully.");
        Ok(())
    }

    pub async fn wait_all_alive(&mut self, timeout: Duration) -> Result<()> {
        // Wait for all of them to startup
        let deadline = Instant::now() + timeout;
        self.wait_for_startup().await?;
        self.wait_for_connectivity(deadline).await?;
        // self.liveness_check(deadline).await?;
        info!("Swarm alive.");
        Ok(())
    }

    pub async fn wait_for_startup(&mut self) -> Result<()> {
        let num_attempts = 30;
        let mut done = vec![false; self.validators.len()];
        for i in 0..num_attempts {
            info!("Wait for startup attempt: {} of {}", i, num_attempts);
            for (node, done) in self.validators.values_mut().zip(done.iter_mut()) {
                if *done {
                    continue;
                }
                match node.health_check().await {
                    Ok(()) => *done = true,

                    Err(HealthCheckError::Unknown(e)) => {
                        return Err(anyhow!(
                            "Node '{}' is not running! Error: {}",
                            node.name(),
                            e
                        ));
                    },
                    Err(HealthCheckError::NotRunning(error)) => {
                        return Err(anyhow!(
                            "Node '{}' is not running! Error: {:?}",
                            node.name(),
                            error
                        ));
                    },
                    Err(HealthCheckError::Failure(e)) => {
                        warn!("health check failure: {}", e);
                        break;
                    },
                }
            }

            // Check if all the nodes have been successfully launched
            if done.iter().all(|status| *status) {
                return Ok(());
            }

            tokio::time::sleep(::std::time::Duration::from_millis(1000)).await;
        }

        Err(anyhow!("Launching Swarm timed out"))
    }

    pub fn add_validator_fullnode(
        &mut self,
        version: &Version,
        config: OverrideNodeConfig,
        validator_peer_id: PeerId,
    ) -> Result<PeerId> {
        let validator = self
            .validators
            .get(&validator_peer_id)
            .ok_or_else(|| anyhow!("no validator with peer_id: {}", validator_peer_id))?;

        let public_network = self
            .public_networks
            .get(&validator_peer_id)
            .ok_or_else(|| anyhow!("no public network with peer_id: {}", validator_peer_id))?;

        if self.fullnodes.contains_key(&validator_peer_id) {
            bail!("VFN for validator {} already configured", validator_peer_id);
        }

        let name = self.node_name_counter.to_string();
        let index = self.node_name_counter;
        self.node_name_counter += 1;
        let fullnode_config = FullnodeNodeConfig::validator_fullnode(
            name,
            self.dir.as_ref(),
            config,
            validator.config(),
            &self.genesis_waypoint,
            &self.genesis,
            public_network,
        )?;

        let version = self.versions.get(version).unwrap();
        let fullnode = LocalNode::new(
            version.to_owned(),
            fullnode_config.name,
            index,
            fullnode_config.dir,
            None,
        )?;

        let peer_id = fullnode.peer_id();
        assert_eq!(peer_id, validator_peer_id);
        fullnode.start()?;

        self.fullnodes.insert(peer_id, fullnode);

        Ok(peer_id)
    }

    fn add_fullnode(&mut self, version: &Version, config: OverrideNodeConfig) -> Result<PeerId> {
        let name = self.node_name_counter.to_string();
        let index = self.node_name_counter;
        self.node_name_counter += 1;
        let fullnode_config = FullnodeNodeConfig::public_fullnode(
            name,
            self.dir.as_ref(),
            config,
            &self.genesis_waypoint,
            &self.genesis,
        )?;

        let version = self.versions.get(version).unwrap();
        let fullnode = LocalNode::new(
            version.to_owned(),
            fullnode_config.name,
            index,
            fullnode_config.dir,
            None,
        )?;

        let peer_id = fullnode.peer_id();
        fullnode.start()?;

        self.fullnodes.insert(peer_id, fullnode);

        Ok(peer_id)
    }

    pub fn root_key(&self) -> Ed25519PrivateKey {
        self.root_key.private_key()
    }

    pub fn chain_id(&self) -> ChainId {
        self.chain_id
    }

    pub fn validator(&self, peer_id: PeerId) -> Option<&LocalNode> {
        self.validators.get(&peer_id)
    }

    pub fn validator_mut(&mut self, peer_id: PeerId) -> Option<&mut LocalNode> {
        self.validators.get_mut(&peer_id)
    }

    pub fn validators(&self) -> impl Iterator<Item = &LocalNode> {
        let mut validators: Vec<&LocalNode> = self.validators.values().collect();
        validators.sort_by_key(|v| v.index()); // Sort by index for consistent ordering
        validators.into_iter()
    }

    pub fn validators_mut(&mut self) -> impl Iterator<Item = &mut LocalNode> {
        let mut validators: Vec<&mut LocalNode> = self.validators.values_mut().collect();
        validators.sort_by_key(|v| v.index()); // Sort by index for consistent ordering
        validators.into_iter()
    }

    pub fn fullnode(&self, peer_id: PeerId) -> Option<&LocalNode> {
        self.fullnodes.get(&peer_id)
    }

    pub fn fullnode_mut(&mut self, peer_id: PeerId) -> Option<&mut LocalNode> {
        self.fullnodes.get_mut(&peer_id)
    }

    pub fn fullnodes(&self) -> impl Iterator<Item = &LocalNode> {
        let mut fullnodes: Vec<&LocalNode> = self.fullnodes.values().collect();
        fullnodes.sort_by_key(|v| v.index()); // Sort by index for consistent ordering
        fullnodes.into_iter()
    }

    pub fn fullnodes_mut(&mut self) -> impl Iterator<Item = &mut LocalNode> {
        let mut fullnodes: Vec<&mut LocalNode> = self.fullnodes.values_mut().collect();
        fullnodes.sort_by_key(|v| v.index()); // Sort by index for consistent ordering
        fullnodes.into_iter()
    }

    pub fn dir(&self) -> &Path {
        self.dir.as_ref()
    }
}

impl Drop for LocalSwarm {
    fn drop(&mut self) {
        // If panicking, persist logs
        if std::env::var("LOCAL_SWARM_SAVE_LOGS").is_ok() || std::thread::panicking() {
            eprintln!("Logs located at {}", self.logs_location());
        }
    }
}

#[async_trait::async_trait]
impl Swarm for LocalSwarm {
    async fn health_check(&self) -> Result<()> {
        Ok(())
    }

    fn validators<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn Validator> + 'a> {
        let mut validators: Vec<_> = self
            .validators
            .values()
            .map(|v| v as &'a dyn Validator)
            .collect();
        validators.sort_by_key(|v| v.index());
        Box::new(validators.into_iter())
    }

    fn validator(&self, id: PeerId) -> Option<&dyn Validator> {
        self.validators.get(&id).map(|v| v as &dyn Validator)
    }

    async fn upgrade_validator(&mut self, id: PeerId, version: &Version) -> Result<()> {
        let version = self
            .versions
            .get(version)
            .cloned()
            .ok_or_else(|| anyhow!("Invalid version: {:?}", version))?;
        let validator = self
            .validators
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Invalid id: {}", id))?;
        validator.upgrade(version)
    }

    fn full_nodes<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn FullNode> + 'a> {
        let mut full_nodes: Vec<_> = self
            .fullnodes
            .values()
            .map(|v| v as &'a dyn FullNode)
            .collect();
        full_nodes.sort_by_key(|n| n.index());
        Box::new(full_nodes.into_iter())
    }

    fn full_node(&self, id: PeerId) -> Option<&dyn FullNode> {
        self.fullnodes.get(&id).map(|v| v as &dyn FullNode)
    }

    fn add_validator(&mut self, _version: &Version, _template: NodeConfig) -> Result<PeerId> {
        todo!()
    }

    fn remove_validator(&mut self, _id: PeerId) -> Result<()> {
        todo!()
    }

    fn add_validator_full_node(
        &mut self,
        version: &Version,
        config: OverrideNodeConfig,
        id: PeerId,
    ) -> Result<PeerId> {
        self.add_validator_fullnode(version, config, id)
    }

    async fn add_full_node(
        &mut self,
        version: &Version,
        config: OverrideNodeConfig,
    ) -> Result<PeerId> {
        self.add_fullnode(version, config)
    }

    fn remove_full_node(&mut self, id: PeerId) -> Result<()> {
        if let Some(fullnode) = self.fullnodes.remove(&id) {
            fullnode.stop();
        }

        Ok(())
    }

    fn versions<'a>(&'a self) -> Box<dyn Iterator<Item = Version> + 'a> {
        Box::new(self.versions.keys().cloned())
    }

    fn chain_info(&self) -> ChainInfo {
        let rest_api_url = self
            .validators()
            .next()
            .unwrap()
            .rest_api_endpoint()
            .to_string();
        let inspection_service_url = self
            .validators()
            .next()
            .unwrap()
            .inspection_service_endpoint()
            .to_string();

        ChainInfo::new(
            self.root_account.clone(),
            rest_api_url,
            inspection_service_url,
            self.chain_id,
        )
    }

    fn logs_location(&mut self) -> String {
        self.dir.persist();
        self.dir.display().to_string()
    }

    async fn inject_chaos(&mut self, _chaos: SwarmChaos) -> Result<()> {
        todo!()
    }

    async fn remove_chaos(&mut self, _chaos: SwarmChaos) -> Result<()> {
        todo!()
    }

    async fn remove_all_chaos(&mut self) -> Result<()> {
        todo!()
    }

    async fn ensure_no_validator_restart(&self) -> Result<()> {
        todo!()
    }

    async fn ensure_no_fullnode_restart(&self) -> Result<()> {
        todo!()
    }

    async fn query_metrics(
        &self,
        _query: &str,
        _time: Option<i64>,
        _timeout: Option<i64>,
    ) -> Result<PromqlResult> {
        todo!()
    }

    async fn query_range_metrics(
        &self,
        _query: &str,
        _start_time: i64,
        _end_time: i64,
        _timeout: Option<i64>,
    ) -> Result<Vec<Sample>> {
        todo!()
    }

    fn chain_info_for_node(&mut self, idx: usize) -> ChainInfo {
        let rest_api_url = self
            .validators()
            .nth(idx)
            .unwrap()
            .rest_api_endpoint()
            .to_string();
        let inspection_service_url = self
            .validators()
            .nth(idx)
            .unwrap()
            .inspection_service_endpoint()
            .to_string();
        ChainInfo::new(
            self.root_account.clone(),
            rest_api_url,
            inspection_service_url,
            self.chain_id,
        )
    }

    fn get_default_pfn_node_config(&self) -> NodeConfig {
        todo!()
    }

    fn has_indexer(&self) -> bool {
        false
    }
}

#[derive(Debug)]
pub struct ActiveNodesGuard {
    counter: Arc<Mutex<usize>>,
    slots: usize,
}

impl ActiveNodesGuard {
    pub async fn grab(slots: usize, counter: Arc<Mutex<usize>>) -> Self {
        let max = num_cpus::get();
        let mut idx = 0;
        loop {
            {
                let mut guard = counter.lock();
                // first check is so that if test needs more slots than cores,
                // we still allow it to run (during low contention)
                if *guard <= 2 || *guard + slots <= max {
                    info!(
                        "Grabbed {} node slots to start test, already active {} swarm nodes",
                        slots, *guard
                    );
                    *guard += slots;
                    drop(guard);
                    return Self { counter, slots };
                }
                idx += 1;
                // log only if idx is power of two, to reduce logs
                if (idx & (idx - 1)) == 0 {
                    info!(
                        "Too many active swarm nodes ({}), max allowed is {}, waiting to start {} new ones",
                        *guard, max, slots,
                    );
                }
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }
}

impl Drop for ActiveNodesGuard {
    fn drop(&mut self) {
        let mut guard = self.counter.lock();
        *guard -= self.slots;
    }
}
