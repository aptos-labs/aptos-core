// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    backend::k8s::node::{K8sNode, REST_API_PORT},
    create_k8s_client, query_sequence_numbers, set_validator_image_tag,
    uninstall_testnet_resources, ChainInfo, FullNode, Node, Result, Swarm, Validator, Version,
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
use k8s_openapi::api::core::v1::Service;
use kube::{
    api::{Api, ListParams},
    client::Client as K8sClient,
};
use std::{collections::HashMap, convert::TryFrom, env, net::TcpListener, str, sync::Arc};
use tokio::{runtime::Runtime, time::Duration};

const VALIDATOR_LB: &str = "validator-lb";
const FULLNODES_LB: &str = "fullnode-lb";
const LOCALHOST: &str = "127.0.0.1";

pub struct K8sSwarm {
    validators: HashMap<PeerId, K8sNode>,
    fullnodes: HashMap<PeerId, K8sNode>,
    root_account: LocalAccount,
    kube_client: K8sClient,
    versions: Arc<HashMap<Version, String>>,
    pub chain_id: ChainId,
    kube_namespace: String,
    keep: bool,
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

        Ok(K8sSwarm {
            validators,
            fullnodes,
            root_account,
            kube_client,
            chain_id: ChainId::new(4),
            versions: Arc::new(versions),
            kube_namespace: kube_namespace.to_string(),
            keep,
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
            validator.sts_name().to_string(),
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
}

/// Amount of time to wait for genesis to complete
pub fn k8s_wait_genesis_strategy() -> impl Iterator<Item = Duration> {
    ExponentWithLimitDelay::new(1000, 10 * 1000, 3 * 60 * 1000)
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

async fn list_services(client: K8sClient, kube_namespace: &str) -> Result<Vec<KubeService>> {
    let node_api: Api<Service> = Api::namespaced(client, kube_namespace);
    let lp = ListParams::default();
    let services = node_api.list(&lp).await?.items;
    services.into_iter().map(KubeService::try_from).collect()
}

fn get_free_port() -> u32 {
    // get a free port
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port() as u32
}

pub(crate) async fn get_validators(
    client: K8sClient,
    image_tag: &str,
    kube_namespace: &str,
    use_port_forward: bool,
) -> Result<HashMap<PeerId, K8sNode>> {
    let services = list_services(client, kube_namespace).await?;
    let validators = services
        .into_iter()
        .filter(|s| s.name.contains(VALIDATOR_LB))
        .map(|s| {
            let mut port = REST_API_PORT;
            let mut ip = s.host_ip.clone();
            if use_port_forward {
                port = get_free_port();
                ip = LOCALHOST.to_string();
            }
            let node_id = parse_node_id(&s.name).expect("error to parse node id");
            let node = K8sNode {
                name: format!("aptos-node-{}-validator", node_id),
                sts_name: parse_node_pod_basename(&s.name).unwrap(),
                // TODO: fetch this from running node
                peer_id: PeerId::random(),
                node_id,
                ip,
                port: port as u32,
                rest_api_port: port as u32,
                dns: s.name,
                version: Version::new(0, image_tag.to_string()),
                namespace: kube_namespace.to_string(),
            };
            (node.peer_id(), node)
        })
        .collect::<HashMap<_, _>>();

    Ok(validators)
}

pub(crate) async fn get_fullnodes(
    client: K8sClient,
    image_tag: &str,
    era: &str,
    kube_namespace: &str,
    use_port_forward: bool,
) -> Result<HashMap<PeerId, K8sNode>> {
    let services = list_services(client, kube_namespace).await?;
    let fullnodes = services
        .into_iter()
        .filter(|s| s.name.contains(FULLNODES_LB))
        .map(|s| {
            let mut port = REST_API_PORT;
            let mut ip = s.host_ip.clone();
            if use_port_forward {
                port = get_free_port();
                ip = LOCALHOST.to_string();
            }
            let node_id = parse_node_id(&s.name).expect("error to parse node id");
            let node = K8sNode {
                name: format!("aptos-node-{}-fullnode", node_id),
                sts_name: format!("{}-e{}", parse_node_pod_basename(&s.name).unwrap(), era),
                // TODO: fetch this from running node
                peer_id: PeerId::random(),
                node_id,
                ip,
                port: port as u32,
                rest_api_port: port as u32,
                dns: s.name,
                version: Version::new(0, image_tag.to_string()),
                namespace: kube_namespace.to_string(),
            };
            (node.peer_id(), node)
        })
        .collect::<HashMap<_, _>>();

    Ok(fullnodes)
}

// gets the node index based on its associated LB service name
// assumes the input is named <RELEASE>-aptos-node-<INDEX>-<validator|fullnode>-lb
fn parse_node_id(s: &str) -> Result<usize> {
    // first get rid of the prefixes
    let v = s.split("aptos-node-").collect::<Vec<&str>>();
    if v.len() < 2 {
        return Err(format_err!("Failed to parse {:?} node id format", s));
    }
    // then get rid of the service name suffix
    let v = v[1].split('-').collect::<Vec<&str>>();
    let idx: usize = v[0].parse().unwrap();
    Ok(idx)
}

// gets the node's underlying STS name based on its associated LB service name
// assumes the input is named <RELEASE>-aptos-node-<INDEX>-<validator|fullnode>-lb
fn parse_node_pod_basename(s: &str) -> Result<String> {
    // first get rid of the prefixes
    let v = s.split("-lb").collect::<Vec<&str>>();
    if v.is_empty() {
        return Err(format_err!("Failed to parse {:?} sts name format", s));
    }
    Ok(v[0].to_string())
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
                info!("Attempting health check: {:?}", node);
                match node.rest_client().get_ledger_information().await {
                    Ok(_) => {
                        info!("Node {} healthy", node.name());
                        Ok(())
                    }
                    Err(x) => {
                        info!("K8s Node {} unhealthy: {}", node.name(), &x);
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
