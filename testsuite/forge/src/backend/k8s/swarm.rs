// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    chaos_schema::{
        Chaos, ChaosConditionType, ChaosStatus, ConditionStatus, NetworkChaos, StressChaos,
    },
    check_for_container_restart, create_k8s_client, delete_all_chaos, get_default_pfn_node_config,
    get_free_port, get_stateful_set_image, install_public_fullnode,
    node::K8sNode,
    prometheus::{self, query_range_with_metadata, query_with_metadata},
    query_sequence_number, set_stateful_set_image_tag, uninstall_testnet_resources, ChainInfo,
    FullNode, K8sApi, Node, Result, Swarm, SwarmChaos, Validator, Version, HAPROXY_SERVICE_SUFFIX,
    REST_API_HAPROXY_SERVICE_PORT, REST_API_SERVICE_PORT,
};
use anyhow::{anyhow, bail, format_err};
use aptos_config::config::{NodeConfig, OverrideNodeConfig};
use aptos_retrier::fixed_retry_strategy;
use aptos_sdk::{
    crypto::ed25519::Ed25519PrivateKey,
    move_types::account_address::AccountAddress,
    types::{chain_id::ChainId, AccountKey, LocalAccount, PeerId},
};
use k8s_openapi::api::{
    apps::v1::StatefulSet,
    core::v1::{ConfigMap, PersistentVolumeClaim, Service},
};
use kube::{
    api::{Api, ListParams},
    client::Client as K8sClient,
};
use log::{debug, info, warn};
use prometheus_http_query::{
    response::{PromqlResult, Sample},
    Client as PrometheusClient,
};
use regex::Regex;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    convert::TryFrom,
    str,
    sync::{atomic::AtomicU32, Arc},
};
use tokio::{
    runtime::{Handle, Runtime},
    task::block_in_place,
    time::Duration,
};

pub struct K8sSwarm {
    validators: HashMap<PeerId, K8sNode>,
    fullnodes: HashMap<PeerId, K8sNode>,
    root_account: Arc<LocalAccount>,
    kube_client: K8sClient,
    versions: Arc<HashMap<Version, String>>,
    pub chain_id: ChainId,
    pub kube_namespace: String,
    keep: bool,
    chaoses: HashSet<SwarmChaos>,
    prom_client: Option<PrometheusClient>,
    era: Option<String>,
    use_port_forward: bool,
    chaos_experiment_ops: Box<dyn ChaosExperimentOps + Send + Sync>,
    has_indexer: bool,
}

impl K8sSwarm {
    pub async fn new<'b>(
        root_key: &[u8],
        image_tag: &str,
        upgrade_image_tag: &str,
        kube_namespace: &str,
        validators: HashMap<AccountAddress, K8sNode>,
        fullnodes: HashMap<AccountAddress, K8sNode>,
        keep: bool,
        era: Option<String>,
        use_port_forward: bool,
        has_indexer: bool,
    ) -> Result<Self> {
        let kube_client = create_k8s_client().await?;

        let client = validators.values().next().unwrap().rest_client();
        let key = load_root_key(root_key);
        let account_key = AccountKey::from_private_key(key);
        let address = aptos_sdk::types::account_config::aptos_test_root_address();
        let sequence_number = query_sequence_number(&client, address).await.map_err(|e| {
            format_err!(
                "query_sequence_number on {:?} for dd account failed: {}",
                client,
                e
            )
        })?;
        let root_account = LocalAccount::new(address, account_key, sequence_number);
        let root_account = Arc::new(root_account);

        let mut versions = HashMap::new();
        let cur_version = Version::new(0, image_tag.to_string());
        let upgrade_version = Version::new(1, upgrade_image_tag.to_string());
        versions.insert(upgrade_version, upgrade_image_tag.to_string());
        versions.insert(cur_version, image_tag.to_string());

        let prom_client = match prometheus::get_prometheus_client().await {
            Ok(p) => Some(p),
            Err(e) => {
                // Fail fast if prometheus is not configured. A test is meaningless if we do not have observability
                bail!("Could not build prometheus client: {}", e);
            },
        };

        let swarm = K8sSwarm {
            validators,
            fullnodes,
            root_account,
            kube_client: kube_client.clone(),
            chain_id: ChainId::new(4),
            versions: Arc::new(versions),
            kube_namespace: kube_namespace.to_string(),
            keep,
            chaoses: HashSet::new(),
            prom_client,
            era,
            use_port_forward,
            chaos_experiment_ops: Box::new(RealChaosExperimentOps {
                kube_client: kube_client.clone(),
                kube_namespace: kube_namespace.to_string(),
            }),
            has_indexer,
        };

        // test hitting the configured prometheus endpoint
        let query = "container_memory_usage_bytes{pod=\"aptos-node-0-validator-0\"}";
        let r = swarm.query_metrics(query, None, None).await?;
        let ivs = r.as_instant().unwrap();
        for iv in ivs {
            info!("container_memory_usage_bytes: {}", iv.sample().value());
        }

        Ok(swarm)
    }

    fn get_rest_api_url(&self, idx: usize) -> String {
        self.validators
            .values()
            .nth(idx)
            .unwrap()
            .rest_api_endpoint()
            .to_string()
    }

    fn get_inspection_service_url(&self, idx: usize) -> String {
        self.validators
            .values()
            .nth(idx)
            .unwrap()
            .inspection_service_endpoint()
            .to_string()
    }

    #[allow(dead_code)]
    fn get_kube_client(&self) -> K8sClient {
        self.kube_client.clone()
    }

    /// Installs a PFN with the given version and node config
    async fn install_public_fullnode_resources<'a>(
        &mut self,
        version: &'a Version,
        node_config: &'a OverrideNodeConfig,
    ) -> Result<(PeerId, K8sNode)> {
        // create APIs
        let stateful_set_api: Arc<K8sApi<_>> = Arc::new(K8sApi::<StatefulSet>::from_client(
            self.get_kube_client(),
            Some(self.kube_namespace.clone()),
        ));
        let configmap_api: Arc<K8sApi<_>> = Arc::new(K8sApi::<ConfigMap>::from_client(
            self.get_kube_client(),
            Some(self.kube_namespace.clone()),
        ));
        let persistent_volume_claim_api: Arc<K8sApi<_>> =
            Arc::new(K8sApi::<PersistentVolumeClaim>::from_client(
                self.get_kube_client(),
                Some(self.kube_namespace.clone()),
            ));
        let service_api: Arc<K8sApi<_>> = Arc::new(K8sApi::<Service>::from_client(
            self.get_kube_client(),
            Some(self.kube_namespace.clone()),
        ));
        let (peer_id, k8snode) = install_public_fullnode(
            stateful_set_api,
            configmap_api,
            persistent_volume_claim_api,
            service_api,
            version,
            node_config,
            self.era
                .as_ref()
                .expect("Installing PFN requires acquiring the current chain era")
                .clone(),
            self.kube_namespace.clone(),
            self.use_port_forward,
            self.fullnodes.len(),
        )
        .await?;
        k8snode.start().await?; // actually start the node. if port-forward is enabled, this is when it gets its ephemeral port
        Ok((peer_id, k8snode))
    }
}

#[async_trait::async_trait]
impl Swarm for K8sSwarm {
    async fn health_check(&self) -> Result<()> {
        let nodes = self.validators.values().collect();
        let unhealthy_nodes = nodes_healthcheck(nodes).await.unwrap();
        if !unhealthy_nodes.is_empty() {
            bail!("Unhealthy nodes: {:?}", unhealthy_nodes)
        }

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

    /// TODO: this should really be a method on Node rather than Swarm
    async fn upgrade_validator(&mut self, id: PeerId, version: &Version) -> Result<()> {
        let validator = self
            .validators
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Invalid id: {}", id))?;
        let version = self
            .versions
            .get(version)
            .cloned()
            .ok_or_else(|| anyhow!("Invalid version: {:?}", version))?;
        // stop the validator first so there is no race on the upgrade
        validator.stop().await?;
        // set the image tag of the StatefulSet spec while there are 0 replicas
        set_stateful_set_image_tag(
            validator.stateful_set_name().to_string(),
            // the container name for the validator in its StatefulSet is "validator"
            "validator".to_string(),
            // extract the image tag from the "version"
            version.to_string(),
            self.kube_namespace.clone(),
        )
        .await?;

        // To ensure that the validator is fully spun back up
        // If port-forward is enabled, this ensures that the pod is back before attempting a port-forward
        validator.start().await?;
        Ok(())
    }

    fn full_nodes<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn FullNode> + 'a> {
        let mut full_nodes: Vec<_> = self
            .fullnodes
            .values()
            .map(|n| n as &'a dyn FullNode)
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
        _version: &Version,
        _config: OverrideNodeConfig,
        _id: PeerId,
    ) -> Result<PeerId> {
        todo!()
    }

    async fn add_full_node(
        &mut self,
        version: &Version,
        config: OverrideNodeConfig,
    ) -> Result<PeerId> {
        self.install_public_fullnode_resources(version, &config)
            .await
            .map(|(peer_id, node)| {
                self.fullnodes.insert(peer_id, node);
                peer_id
            })
    }

    fn remove_full_node(&mut self, _id: PeerId) -> Result<()> {
        todo!()
    }

    fn versions<'a>(&'a self) -> Box<dyn Iterator<Item = Version> + 'a> {
        Box::new(self.versions.keys().cloned())
    }

    fn chain_info(&self) -> ChainInfo {
        let rest_api_url = self.get_rest_api_url(0);
        let inspection_service_url = self.get_inspection_service_url(0);
        ChainInfo::new(
            self.root_account.clone(),
            rest_api_url,
            inspection_service_url,
            self.chain_id,
        )
    }

    // returns a kubectl logs command to retrieve the logs manually
    // and instructions to check the actual live logs location from fgi
    fn logs_location(&mut self) -> String {
        "See fgi output for more information.".to_string()
    }

    async fn inject_chaos(&mut self, chaos: SwarmChaos) -> Result<()> {
        self.inject_swarm_chaos(&chaos)?;
        self.chaoses.insert(chaos);
        self.chaos_experiment_ops
            .ensure_chaos_experiments_active()
            .await?;

        Ok(())
    }

    async fn remove_chaos(&mut self, chaos: SwarmChaos) -> Result<()> {
        self.chaos_experiment_ops
            .ensure_chaos_experiments_active()
            .await?;

        if self.chaoses.remove(&chaos) {
            self.remove_swarm_chaos(&chaos)?;
        } else {
            bail!("Chaos {:?} not found", chaos);
        }
        Ok(())
    }

    async fn remove_all_chaos(&mut self) -> Result<()> {
        self.chaos_experiment_ops
            .ensure_chaos_experiments_active()
            .await?;

        // try removing all existing chaoses
        for chaos in self.chaoses.clone() {
            self.remove_swarm_chaos(&chaos)?;
        }
        // force remove all others
        delete_all_chaos(&self.kube_namespace)?;

        self.chaoses.clear();
        Ok(())
    }

    async fn ensure_no_validator_restart(&self) -> Result<()> {
        for validator in &self.validators {
            check_for_container_restart(
                &self.kube_client,
                &self.kube_namespace.clone(),
                validator.1.stateful_set_name(),
            )
            .await?;
        }
        info!("Found no validator restarts");
        Ok(())
    }

    async fn ensure_no_fullnode_restart(&self) -> Result<()> {
        for fullnode in &self.fullnodes {
            check_for_container_restart(
                &self.kube_client,
                &self.kube_namespace.clone(),
                fullnode.1.stateful_set_name(),
            )
            .await?;
        }
        info!("Found no fullnode restarts");
        Ok(())
    }

    async fn query_metrics(
        &self,
        query: &str,
        time: Option<i64>,
        timeout: Option<i64>,
    ) -> Result<PromqlResult> {
        if let Some(c) = &self.prom_client {
            let mut labels_map = BTreeMap::new();
            labels_map.insert("namespace".to_string(), self.kube_namespace.clone());
            return query_with_metadata(c, query, time, timeout, &labels_map).await;
        }
        bail!("No prom client");
    }

    async fn query_range_metrics(
        &self,
        query: &str,
        start_time: i64,
        end_time: i64,
        timeout: Option<i64>,
    ) -> Result<Vec<Sample>> {
        if let Some(c) = &self.prom_client {
            let mut labels_map = BTreeMap::new();
            labels_map.insert("namespace".to_string(), self.kube_namespace.clone());
            return query_range_with_metadata(
                c,
                query,
                start_time,
                end_time,
                30.0,
                timeout,
                &labels_map,
            )
            .await;
        }
        bail!("No prom client");
    }

    fn chain_info_for_node(&mut self, idx: usize) -> ChainInfo {
        let rest_api_url = self.get_rest_api_url(idx);
        let inspection_service_url = self.get_inspection_service_url(idx);
        ChainInfo::new(
            self.root_account.clone(),
            rest_api_url,
            inspection_service_url,
            self.chain_id,
        )
    }

    fn get_default_pfn_node_config(&self) -> NodeConfig {
        get_default_pfn_node_config()
    }

    fn has_indexer(&self) -> bool {
        self.has_indexer
    }
}

/// Amount of time to wait for genesis to complete
pub fn k8s_wait_genesis_strategy() -> impl Iterator<Item = Duration> {
    // retry every 10 seconds for 10 minutes
    fixed_retry_strategy(10 * 1000, 60)
}

/// Amount of time to wait for nodes to spin up, from provisioning to API ready
pub fn k8s_wait_nodes_strategy() -> impl Iterator<Item = Duration> {
    // retry every 10 seconds for 20 minutes
    fixed_retry_strategy(10 * 1000, 120)
}

async fn list_stateful_sets(client: K8sClient, kube_namespace: &str) -> Result<Vec<StatefulSet>> {
    let stateful_set_api: Api<StatefulSet> = Api::namespaced(client, kube_namespace);
    let lp = ListParams::default();
    let stateful_sets = stateful_set_api.list(&lp).await?.items;
    Ok(stateful_sets)
}

/// Check if the stateful set labels match the given labels
fn stateful_set_labels_matches(sts: &StatefulSet, labels: &BTreeMap<String, String>) -> bool {
    if sts.metadata.labels.is_none() {
        return false;
    }
    let sts_labels = sts
        .metadata
        .labels
        .as_ref()
        .expect("Failed to get StatefulSet labels");
    labels.iter().all(|(k, v)| {
        let truncated_k = k.chars().take(63).collect::<String>();
        let truncated_v = v.chars().take(63).collect::<String>();
        // warn if the label is truncated
        if truncated_k != *k || truncated_v != *v {
            warn!(
                "Label truncated during search: {} -> {}, {} -> {}",
                k, truncated_k, v, truncated_v
            );
        }
        sts_labels.get(&truncated_k) == Some(&truncated_v)
    })
}

fn parse_service_name_from_stateful_set_name(
    stateful_set_name: &str,
    enable_haproxy: bool,
) -> String {
    let re = Regex::new(r"(aptos-node-\d+)-(validator|fullnode)").unwrap();
    let cap = re.captures(stateful_set_name).unwrap();
    let service_base_name = format!("{}-{}", &cap[1], &cap[2]);
    if enable_haproxy {
        format!("{}-{}", &service_base_name, HAPROXY_SERVICE_SUFFIX)
    } else {
        service_base_name
    }
}

fn get_k8s_node_from_stateful_set(
    sts: &StatefulSet,
    enable_haproxy: bool,
    use_port_forward: bool,
) -> K8sNode {
    let stateful_set_name = sts.metadata.name.as_ref().unwrap();
    // If HAProxy is enabled, use its Service name. Otherwise the Service name matches the StatefulSet name
    let mut service_name =
        parse_service_name_from_stateful_set_name(stateful_set_name, enable_haproxy);

    // the full service name includes the namespace
    let namespace = sts.metadata.namespace.as_ref().unwrap();

    // if we're not using port-forward and expecting to hit the service directly, we should use the full service name
    // since the test runner may be in a separate namespace
    if !use_port_forward {
        service_name = format!("{}.{}.svc", &service_name, &namespace);
    }

    // Append the cluster name if its a multi-cluster deployment
    let service_name = if let Some(target_cluster_name) = sts
        .metadata
        .labels
        .as_ref()
        .and_then(|labels| labels.get("multicluster/targetcluster"))
    {
        format!("{}.{}", &service_name, &target_cluster_name)
    } else {
        service_name
    };

    // If HAProxy is enabled, use the port on its Service. Otherwise use the port on the validator Service
    let mut rest_api_port = if enable_haproxy {
        REST_API_HAPROXY_SERVICE_PORT
    } else {
        REST_API_SERVICE_PORT
    };

    if use_port_forward {
        rest_api_port = get_free_port();
    }
    let index = parse_node_index(stateful_set_name).expect("error to parse node index");
    let node_type = parse_node_type(stateful_set_name);

    // Extract the image tag from the StatefulSet spec
    let image_tag = get_stateful_set_image(sts)
        .expect("Failed to get StatefulSet image")
        .tag;

    K8sNode {
        name: format!("{}-{}", &node_type, index),
        stateful_set_name: stateful_set_name.clone(),
        // TODO: fetch this from running node
        peer_id: PeerId::random(),
        index,
        service_name,
        rest_api_port: AtomicU32::new(rest_api_port),
        version: Version::new(0, image_tag),
        namespace: namespace.to_string(),
        haproxy_enabled: enable_haproxy,
        port_forward_enabled: use_port_forward,
    }
}

pub(crate) async fn get_validators(
    client: K8sClient,
    kube_namespace: &str,
    use_port_forward: bool,
    enable_haproxy: bool,
) -> Result<HashMap<PeerId, K8sNode>> {
    let stateful_sets = list_stateful_sets(client, kube_namespace).await?;
    let validators = stateful_sets
        .into_iter()
        .filter(|sts| {
            stateful_set_labels_matches(
                sts,
                &BTreeMap::from([
                    (
                        "app.kubernetes.io/name".to_string(),
                        "validator".to_string(),
                    ),
                    (
                        "app.kubernetes.io/part-of".to_string(),
                        "aptos-node".to_string(),
                    ),
                ]),
            )
        })
        .map(|sts| {
            let node = get_k8s_node_from_stateful_set(&sts, enable_haproxy, use_port_forward);
            (node.peer_id(), node)
        })
        .collect::<HashMap<_, _>>();

    Ok(validators)
}

pub(crate) async fn get_validator_fullnodes(
    client: K8sClient,
    kube_namespace: &str,
    use_port_forward: bool,
    enable_haproxy: bool,
) -> Result<HashMap<PeerId, K8sNode>> {
    let stateful_sets = list_stateful_sets(client, kube_namespace).await?;
    let fullnodes = stateful_sets
        .into_iter()
        .filter(|sts| {
            stateful_set_labels_matches(
                sts,
                &BTreeMap::from([
                    ("app.kubernetes.io/name".to_string(), "fullnode".to_string()),
                    (
                        "app.kubernetes.io/part-of".to_string(),
                        "aptos-node".to_string(),
                    ),
                ]),
            )
        })
        .map(|sts| {
            let node = get_k8s_node_from_stateful_set(&sts, enable_haproxy, use_port_forward);
            (node.peer_id(), node)
        })
        .collect::<HashMap<_, _>>();

    Ok(fullnodes)
}

/// Given a string like the StatefulSet name or Service name, parse the node type,
/// whether it's a validator or fullnode
fn parse_node_type(s: &str) -> String {
    let re = Regex::new(r"(validator|fullnode)").unwrap();
    let cap = re.captures(s).unwrap();
    cap[1].to_string()
}

// gets the node index based on its associated statefulset name
// e.g. aptos-node-<idx>-validator
// e.g. aptos-node-<idx>-fullnode-e<era>
fn parse_node_index(s: &str) -> Result<usize> {
    // first get rid of the prefixes
    let v = s.split("aptos-node-").collect::<Vec<&str>>();
    if v.len() < 2 {
        return Err(format_err!("Failed to parse {:?} node id format", s));
    }
    // then get rid of the node type suffix
    let v = v[1].split('-').collect::<Vec<&str>>();
    let idx: usize = v[0].parse().unwrap();
    Ok(idx)
}

fn load_root_key(root_key_bytes: &[u8]) -> Ed25519PrivateKey {
    Ed25519PrivateKey::try_from(root_key_bytes).unwrap()
}

pub async fn nodes_healthcheck(nodes: Vec<&K8sNode>) -> Result<Vec<String>> {
    let mut unhealthy_nodes = vec![];

    // TODO(rustielin): do all nodes healthchecks in parallel
    for node in nodes {
        // perform healthcheck with retry, returning unhealthy
        let node_name = node.name().to_string();
        let check = aptos_retrier::retry_async(k8s_wait_nodes_strategy(), || {
            Box::pin(async move {
                match node.rest_client().get_ledger_information().await {
                    Ok(res) => {
                        let version = res.inner().version;
                        // ensure a threshold liveness for each node
                        // we want to guarantee node is making progress without spinning too long
                        if version > 100 {
                            info!("Node {} healthy @ version {} > 100", node.name(), version);
                            return Ok(());
                        }
                        info!("Node {} @ version {}", node.name(), version);
                        bail!(
                            "Node {} unhealthy: REST API returned version 0",
                            node.name()
                        );
                    },
                    Err(err) => {
                        let err = anyhow::Error::from(err);
                        info!("Node {} unhealthy: {}", node.name(), &err);
                        Err(err)
                    },
                }
            })
        })
        .await;
        if check.is_err() {
            unhealthy_nodes.push(node_name);
        }
    }
    if !unhealthy_nodes.is_empty() {
        debug!("Unhealthy validators after cleanup: {:?}", unhealthy_nodes);
    }

    Ok(unhealthy_nodes)
}

impl Drop for K8sSwarm {
    fn drop(&mut self) {
        if !self.keep {
            let fut = uninstall_testnet_resources(self.kube_namespace.clone());
            match Handle::try_current() {
                Ok(handle) => block_in_place(move || handle.block_on(fut).unwrap()),
                Err(_err) => {
                    let runtime = Runtime::new().unwrap();
                    runtime.block_on(fut).unwrap();
                },
            }
        } else {
            println!("Keeping kube_namespace {}", self.kube_namespace);
        }
    }
}

#[async_trait::async_trait]
trait ChaosExperimentOps {
    async fn list_network_chaos(&self) -> Result<Vec<NetworkChaos>>;
    async fn list_stress_chaos(&self) -> Result<Vec<StressChaos>>;

    async fn ensure_chaos_experiments_active(&self) -> Result<()> {
        let timeout_duration = Duration::from_secs(600); // 10 minutes
        let polling_interval = Duration::from_secs(10);

        tokio::time::timeout(timeout_duration, async {
            loop {
                match self.are_chaos_experiments_active().await {
                    Ok(true) => {
                        info!("Chaos experiments are active");
                        return Ok(());
                    },
                    Ok(false) => {
                        info!("Chaos experiments are not active, retrying...");
                    },
                    Err(e) => {
                        warn!(
                            "Error while checking chaos experiments status: {}. Retrying...",
                            e
                        );
                    },
                }
                tokio::time::sleep(polling_interval).await;
            }
        })
        .await
        .map_err(|e| {
            anyhow!(
                "Timed out waiting for chaos experiments to be active: {}",
                e
            )
        })?
    }

    /// Checks if all chaos experiments are active
    async fn are_chaos_experiments_active(&self) -> Result<bool> {
        let (network_chaoses, stress_chaoses) =
            tokio::join!(self.list_network_chaos(), self.list_stress_chaos());

        let chaoses: Vec<Chaos> = network_chaoses?
            .into_iter()
            .map(Chaos::Network)
            .chain(stress_chaoses?.into_iter().map(Chaos::Stress))
            .collect();

        Ok(!chaoses.is_empty()
            && chaoses.iter().all(|chaos| match chaos {
                Chaos::Network(network_chaos) => check_all_injected(&network_chaos.status),
                Chaos::Stress(stress_chaos) => check_all_injected(&stress_chaos.status),
            }))
    }
}

fn check_all_injected(status: &Option<ChaosStatus>) -> bool {
    status
        .as_ref()
        .and_then(|status| status.conditions.as_ref())
        .is_some_and(|conditions| {
            conditions.iter().any(|c| {
                c.r#type == ChaosConditionType::AllInjected && c.status == ConditionStatus::True
            }) && conditions.iter().any(|c| {
                c.r#type == ChaosConditionType::Selected && c.status == ConditionStatus::True
            })
        })
}

#[allow(dead_code)]
struct MockChaosExperimentOps {
    network_chaos: Vec<NetworkChaos>,
    stress_chaos: Vec<StressChaos>,
}

#[async_trait::async_trait]
impl ChaosExperimentOps for MockChaosExperimentOps {
    async fn list_network_chaos(&self) -> Result<Vec<NetworkChaos>> {
        Ok(self.network_chaos.clone())
    }

    async fn list_stress_chaos(&self) -> Result<Vec<StressChaos>> {
        Ok(self.stress_chaos.clone())
    }
}

struct RealChaosExperimentOps {
    kube_client: K8sClient,
    kube_namespace: String,
}

#[async_trait::async_trait]
impl ChaosExperimentOps for RealChaosExperimentOps {
    async fn list_network_chaos(&self) -> Result<Vec<NetworkChaos>> {
        let network_chaos_api: Api<NetworkChaos> =
            Api::namespaced(self.kube_client.clone(), &self.kube_namespace);
        let lp = ListParams::default();
        let network_chaoses = network_chaos_api.list(&lp).await?.items;
        Ok(network_chaoses)
    }

    async fn list_stress_chaos(&self) -> Result<Vec<StressChaos>> {
        let stress_chaos_api: Api<StressChaos> =
            Api::namespaced(self.kube_client.clone(), &self.kube_namespace);
        let lp = ListParams::default();
        let stress_chaoses = stress_chaos_api.list(&lp).await?.items;
        Ok(stress_chaoses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chaos_schema::ChaosCondition;
    use kube::api::ObjectMeta;

    #[test]
    fn test_parse_service_name_from_stateful_set_name() {
        let validator_sts_name = "aptos-node-19-validator";
        let validator_service_name =
            parse_service_name_from_stateful_set_name(validator_sts_name, false);
        assert_eq!("aptos-node-19-validator", &validator_service_name);
        // with haproxy
        let validator_service_name =
            parse_service_name_from_stateful_set_name(validator_sts_name, true);
        assert_eq!("aptos-node-19-validator-lb", &validator_service_name);

        let fullnode_sts_name = "aptos-node-0-fullnode-eforge195";
        let fullnode_service_name =
            parse_service_name_from_stateful_set_name(fullnode_sts_name, false);
        assert_eq!("aptos-node-0-fullnode", &fullnode_service_name);
        // with haproxy
        let fullnode_service_name =
            parse_service_name_from_stateful_set_name(fullnode_sts_name, true);
        assert_eq!("aptos-node-0-fullnode-lb", &fullnode_service_name);
    }

    async fn create_chaos_experiments(
        network_status: ConditionStatus,
        stress_status: ConditionStatus,
    ) -> (Vec<NetworkChaos>, Vec<StressChaos>) {
        let network_chaos = NetworkChaos {
            status: Some(ChaosStatus {
                conditions: Some(vec![
                    ChaosCondition {
                        r#type: ChaosConditionType::AllInjected,
                        status: network_status.clone(),
                    },
                    ChaosCondition {
                        r#type: ChaosConditionType::Selected,
                        status: network_status,
                    },
                ]),
            }),
            ..NetworkChaos::new("test", Default::default())
        };
        let stress_chaos = StressChaos {
            status: Some(ChaosStatus {
                conditions: Some(vec![
                    ChaosCondition {
                        r#type: ChaosConditionType::AllInjected,
                        status: stress_status.clone(),
                    },
                    ChaosCondition {
                        r#type: ChaosConditionType::Selected,
                        status: stress_status,
                    },
                ]),
            }),
            ..StressChaos::new("test", Default::default())
        };
        (vec![network_chaos], vec![stress_chaos])
    }

    #[tokio::test]
    async fn test_chaos_experiments_active() {
        // No experiments active
        let chaos_ops = MockChaosExperimentOps {
            network_chaos: vec![],
            stress_chaos: vec![],
        };
        assert!(!chaos_ops.are_chaos_experiments_active().await.unwrap());

        // Only network chaos active
        let (network_chaos, stress_chaos) =
            create_chaos_experiments(ConditionStatus::True, ConditionStatus::False).await;
        let chaos_ops = MockChaosExperimentOps {
            network_chaos,
            stress_chaos,
        };
        assert!(!chaos_ops.are_chaos_experiments_active().await.unwrap());

        // Both network and stress chaos active
        let (network_chaos, stress_chaos) =
            create_chaos_experiments(ConditionStatus::True, ConditionStatus::True).await;
        let chaos_ops = MockChaosExperimentOps {
            network_chaos,
            stress_chaos,
        };
        assert!(chaos_ops.are_chaos_experiments_active().await.unwrap());
    }

    #[test]
    fn test_stateful_set_labels_matches() {
        // Create a StatefulSet with some labels
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), "validator".to_string());
        labels.insert("component".to_string(), "blockchain".to_string());

        let sts = StatefulSet {
            metadata: ObjectMeta {
                labels: Some(labels),
                ..Default::default()
            },
            ..Default::default()
        };

        // All labels match
        let mut match_labels = BTreeMap::new();
        match_labels.insert("app".to_string(), "validator".to_string());
        match_labels.insert("component".to_string(), "blockchain".to_string());
        assert!(stateful_set_labels_matches(&sts, &match_labels));

        // Subset of labels match
        let mut match_labels = BTreeMap::new();
        match_labels.insert("app".to_string(), "validator".to_string());
        assert!(stateful_set_labels_matches(&sts, &match_labels));

        // One label doesn't match
        let mut match_labels = BTreeMap::new();
        match_labels.insert("app".to_string(), "validator".to_string());
        match_labels.insert("component".to_string(), "database".to_string());
        assert!(!stateful_set_labels_matches(&sts, &match_labels));

        // Extra label in match_labels
        let mut match_labels = BTreeMap::new();
        match_labels.insert("app".to_string(), "validator".to_string());
        match_labels.insert("component".to_string(), "blockchain".to_string());
        match_labels.insert("extra".to_string(), "label".to_string());
        assert!(!stateful_set_labels_matches(&sts, &match_labels));

        // Empty match_labels
        let match_labels = BTreeMap::new();
        assert!(stateful_set_labels_matches(&sts, &match_labels));

        // StatefulSet with no labels
        let sts_no_labels = StatefulSet {
            metadata: ObjectMeta {
                labels: None,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut match_labels = BTreeMap::new();
        match_labels.insert("app".to_string(), "validator".to_string());
        assert!(!stateful_set_labels_matches(&sts_no_labels, &match_labels));

        // StatefulSet with truncated labels
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), "validator".to_string());
        // component label is truncated to 63 characters
        labels.insert(
            "component".to_string(),
            "blockchain"
                .to_string()
                .repeat(10)
                .chars()
                .take(63)
                .collect::<String>(),
        );

        let sts_truncated_labels = StatefulSet {
            metadata: ObjectMeta {
                labels: Some(labels),
                ..Default::default()
            },
            ..Default::default()
        };
        let mut match_labels = BTreeMap::new();
        // we try to match with the full label, which we dont know if it's truncated or not
        match_labels.insert(
            "component".to_string(),
            "blockchain"
                .to_string()
                .repeat(10)
                .chars()
                .collect::<String>(),
        );
        // it should match because the labels are the same when truncated
        assert!(stateful_set_labels_matches(
            &sts_truncated_labels,
            &match_labels
        ));
    }
}
