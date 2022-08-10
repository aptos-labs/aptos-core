// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    chaos, create_k8s_client, get_free_port,
    node::{K8sNode, REST_API_HAPROXY_SERVICE_PORT, REST_API_SERVICE_PORT},
    prometheus::{self, query_with_metadata},
    query_sequence_numbers, set_validator_image_tag, uninstall_testnet_resources, ChainInfo,
    FullNode, Node, Result, Swarm, SwarmChaos, Validator, Version,
};
use ::aptos_logger::*;
use anyhow::{anyhow, bail, format_err};
use aptos_config::config::NodeConfig;
use aptos_retrier::ExponentWithLimitDelay;
use aptos_sdk::{
    crypto::ed25519::Ed25519PrivateKey,
    move_types::account_address::AccountAddress,
    types::{chain_id::ChainId, AccountKey, LocalAccount, PeerId},
};
use k8s_openapi::api::{apps::v1::StatefulSet, core::v1::Service};
use kube::{
    api::{Api, ListParams},
    client::Client as K8sClient,
};
use prometheus_http_query::{response::PromqlResult, Client as PrometheusClient};
use regex::Regex;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    convert::TryFrom,
    env, str,
    sync::Arc,
};
use tokio::{runtime::Runtime, time::Duration};

pub const VALIDATOR_SERVICE_SUFFIX: &str = "validator";
pub const FULLNODE_SERVICE_SUFFIX: &str = "fullnode";
pub const VALIDATOR_HAPROXY_SERVICE_SUFFIX: &str = "validator-lb";
pub const FULLNODE_HAPROXY_SERVICE_SUFFIX: &str = "fullnode-lb";
pub const HAPROXY_SERVICE_SUFFIX: &str = "lb";

pub struct K8sSwarm {
    validators: HashMap<PeerId, K8sNode>,
    fullnodes: HashMap<PeerId, K8sNode>,
    root_account: LocalAccount,
    kube_client: K8sClient,
    versions: Arc<HashMap<Version, String>>,
    pub chain_id: ChainId,
    kube_namespace: String,
    keep: bool,
    chaoses: HashSet<SwarmChaos>,
    prom_client: Option<PrometheusClient>,
}

impl K8sSwarm {
    pub async fn new(
        root_key: &[u8],
        image_tag: &str,
        base_image_tag: &str,
        kube_namespace: &str,
        validators: HashMap<AccountAddress, K8sNode>,
        fullnodes: HashMap<AccountAddress, K8sNode>,
        keep: bool,
    ) -> Result<Self> {
        let kube_client = create_k8s_client().await;

        let client = validators.values().next().unwrap().rest_client();
        let key = load_root_key(root_key);
        let account_key = AccountKey::from_private_key(key);
        let address = aptos_sdk::types::account_config::aptos_root_address();
        let sequence_number = query_sequence_numbers(&client, &[address])
            .await
            .map_err(|e| {
                format_err!(
                    "query_sequence_numbers on {:?} for dd account failed: {}",
                    client,
                    e
                )
            })?[0];
        let root_account = LocalAccount::new(address, account_key, sequence_number);

        let mut versions = HashMap::new();
        let base_version = Version::new(0, base_image_tag.to_string());
        let cur_version = Version::new(1, image_tag.to_string());
        versions.insert(base_version, base_image_tag.to_string());
        versions.insert(cur_version, image_tag.to_string());

        let prom_client = match prometheus::get_prometheus_client() {
            Ok(p) => Some(p),
            Err(e) => {
                info!("Could not build prometheus client: {}", e);
                None
            }
        };

        Ok(K8sSwarm {
            validators,
            fullnodes,
            root_account,
            kube_client,
            chain_id: ChainId::new(4),
            versions: Arc::new(versions),
            kube_namespace: kube_namespace.to_string(),
            keep,
            chaoses: HashSet::new(),
            prom_client,
        })
    }

    fn get_rest_api_url(&self) -> String {
        self.validators
            .values()
            .next()
            .unwrap()
            .rest_api_endpoint()
            .to_string()
    }

    #[allow(dead_code)]
    fn get_kube_client(&self) -> K8sClient {
        self.kube_client.clone()
    }
}

#[async_trait::async_trait]
impl Swarm for K8sSwarm {
    async fn health_check(&mut self) -> Result<()> {
        let nodes = self.validators.values().collect();
        let unhealthy_nodes = nodes_healthcheck(nodes).await.unwrap();
        if !unhealthy_nodes.is_empty() {
            bail!("Unhealthy nodes: {:?}", unhealthy_nodes)
        }

        Ok(())
    }

    fn validators<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn Validator> + 'a> {
        Box::new(self.validators.values().map(|v| v as &'a dyn Validator))
    }

    fn validators_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut dyn Validator> + 'a> {
        Box::new(
            self.validators
                .values_mut()
                .map(|v| v as &'a mut dyn Validator),
        )
    }

    fn validator(&self, id: PeerId) -> Option<&dyn Validator> {
        self.validators.get(&id).map(|v| v as &dyn Validator)
    }

    fn validator_mut(&mut self, id: PeerId) -> Option<&mut dyn Validator> {
        self.validators
            .get_mut(&id)
            .map(|v| v as &mut dyn Validator)
    }

    fn upgrade_validator(&mut self, id: PeerId, version: &Version) -> Result<()> {
        let validator = self
            .validators
            .get_mut(&id)
            .ok_or_else(|| anyhow!("Invalid id: {}", id))?;
        let version = self
            .versions
            .get(version)
            .cloned()
            .ok_or_else(|| anyhow!("Invalid version: {:?}", version))?;
        set_validator_image_tag(
            validator.stateful_set_name().to_string(),
            version,
            self.kube_namespace.clone(),
        )
    }

    fn full_nodes<'a>(&'a self) -> Box<dyn Iterator<Item = &'a dyn FullNode> + 'a> {
        Box::new(self.fullnodes.values().map(|v| v as &'a dyn FullNode))
    }

    fn full_nodes_mut<'a>(&'a mut self) -> Box<dyn Iterator<Item = &'a mut dyn FullNode> + 'a> {
        Box::new(
            self.fullnodes
                .values_mut()
                .map(|v| v as &'a mut dyn FullNode),
        )
    }

    fn full_node(&self, id: PeerId) -> Option<&dyn FullNode> {
        self.fullnodes.get(&id).map(|v| v as &dyn FullNode)
    }

    fn full_node_mut(&mut self, id: PeerId) -> Option<&mut dyn FullNode> {
        self.fullnodes.get_mut(&id).map(|v| v as &mut dyn FullNode)
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
        _template: NodeConfig,
        _id: PeerId,
    ) -> Result<PeerId> {
        todo!()
    }

    fn add_full_node(&mut self, _version: &Version, _template: NodeConfig) -> Result<PeerId> {
        todo!()
    }

    fn remove_full_node(&mut self, _id: PeerId) -> Result<()> {
        todo!()
    }

    fn versions<'a>(&'a self) -> Box<dyn Iterator<Item = Version> + 'a> {
        Box::new(self.versions.keys().cloned())
    }

    fn chain_info(&mut self) -> ChainInfo<'_> {
        let rest_api_url = self.get_rest_api_url();
        ChainInfo::new(&mut self.root_account, rest_api_url, self.chain_id)
    }

    // returns a kubectl logs command to retrieve the logs manually
    // and instructions to check the actual live logs location from fgi
    fn logs_location(&mut self) -> String {
        "See fgi output for more information.".to_string()
    }

    fn inject_chaos(&mut self, chaos: SwarmChaos) -> Result<()> {
        chaos::inject_swarm_chaos(&self.kube_namespace, &chaos)?;
        self.chaoses.insert(chaos);
        Ok(())
    }

    fn remove_chaos(&mut self, chaos: SwarmChaos) -> Result<()> {
        if self.chaoses.remove(&chaos) {
            chaos::remove_swarm_chaos(&self.kube_namespace, &chaos)?;
        } else {
            bail!("Chaos {:?} not found", chaos);
        }
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
            return query_with_metadata(c, query, time, timeout, labels_map).await;
        }
        bail!("No prom client");
    }
}

/// Amount of time to wait for genesis to complete
pub fn k8s_wait_genesis_strategy() -> impl Iterator<Item = Duration> {
    ExponentWithLimitDelay::new(1000, 10 * 1000, 10 * 60 * 1000)
}

/// Amount of time to wait for nodes to respond on the REST API
pub fn k8s_wait_nodes_strategy() -> impl Iterator<Item = Duration> {
    ExponentWithLimitDelay::new(1000, 10 * 1000, 15 * 60 * 1000)
}

#[derive(Clone, Debug)]
pub struct KubeService {
    pub name: String,
    pub host_ip: String,
}

impl TryFrom<Service> for KubeService {
    type Error = anyhow::Error;

    fn try_from(service: Service) -> Result<Self, Self::Error> {
        let metadata = service.metadata;
        let name = metadata
            .name
            .ok_or_else(|| format_err!("node name not found"))?;
        let spec = service
            .spec
            .ok_or_else(|| format_err!("spec not found for node"))?;
        let host_ip = spec.cluster_ip.unwrap_or_default();
        Ok(Self { name, host_ip })
    }
}

async fn list_stateful_sets(client: K8sClient, kube_namespace: &str) -> Result<Vec<StatefulSet>> {
    let stateful_set_api: Api<StatefulSet> = Api::namespaced(client, kube_namespace);
    let lp = ListParams::default();
    let stateful_sets = stateful_set_api.list(&lp).await?.items;
    Ok(stateful_sets)
}

fn stateful_set_name_matches(sts: &StatefulSet, suffix: &str) -> bool {
    if let Some(s) = sts.metadata.name.as_ref() {
        s.contains(suffix)
    } else {
        false
    }
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
    let image_tag = sts
        .spec
        .as_ref()
        .unwrap()
        .template
        .spec
        .as_ref()
        .unwrap()
        .containers[0]
        .image
        .as_ref()
        .unwrap()
        .split(':')
        .collect::<Vec<&str>>()[1];

    K8sNode {
        name: format!("{}-{}", &node_type, index),
        stateful_set_name: stateful_set_name.clone(),
        // TODO: fetch this from running node
        peer_id: PeerId::random(),
        index,
        service_name,
        rest_api_port,
        version: Version::new(0, image_tag.to_string()),
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
        .filter(|sts| stateful_set_name_matches(sts, "validator"))
        .map(|sts| {
            let node = get_k8s_node_from_stateful_set(&sts, enable_haproxy, use_port_forward);
            (node.peer_id(), node)
        })
        .collect::<HashMap<_, _>>();

    Ok(validators)
}

pub(crate) async fn get_fullnodes(
    client: K8sClient,
    kube_namespace: &str,
    use_port_forward: bool,
    enable_haproxy: bool,
) -> Result<HashMap<PeerId, K8sNode>> {
    let stateful_sets = list_stateful_sets(client, kube_namespace).await?;
    let fullnodes = stateful_sets
        .into_iter()
        .filter(|sts| stateful_set_name_matches(sts, "fullnode"))
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
                    }
                    Err(x) => {
                        info!("Node {} unhealthy: {}", node.name(), &x);
                        Err(x)
                    }
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
        let runtime = Runtime::new().unwrap();
        if !self.keep {
            runtime
                .block_on(uninstall_testnet_resources(self.kube_namespace.clone()))
                .unwrap();
        } else {
            println!("Keeping kube_namespace {}", self.kube_namespace);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
