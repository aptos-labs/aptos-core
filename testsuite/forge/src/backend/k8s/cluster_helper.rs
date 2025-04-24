// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::GENESIS_HELM_RELEASE_NAME;
use crate::{
    get_validator_fullnodes, get_validators, k8s_wait_nodes_strategy, nodes_healthcheck,
    wait_stateful_set, ForgeDeployerManager, ForgeRunnerMode, GenesisConfigFn, K8sApi, K8sNode,
    NodeConfigFn, ReadWrite, Result, APTOS_NODE_HELM_RELEASE_NAME, DEFAULT_ROOT_KEY,
    DEFAULT_TEST_SUITE_NAME, DEFAULT_USERNAME, FORGE_KEY_SEED,
    FORGE_TESTNET_DEPLOYER_DOCKER_IMAGE_REPO, FULLNODE_HAPROXY_SERVICE_SUFFIX,
    FULLNODE_SERVICE_SUFFIX, HELM_BIN, KUBECTL_BIN, MANAGEMENT_CONFIGMAP_PREFIX,
    NAMESPACE_CLEANUP_THRESHOLD_SECS, POD_CLEANUP_THRESHOLD_SECS, VALIDATOR_HAPROXY_SERVICE_SUFFIX,
    VALIDATOR_SERVICE_SUFFIX,
};
use again::RetryPolicy;
use anyhow::{anyhow, bail, format_err};
use aptos_logger::info;
use aptos_sdk::types::PeerId;
use k8s_openapi::api::{
    apps::v1::{Deployment, StatefulSet},
    batch::v1::Job,
    core::v1::{ConfigMap, Namespace, PersistentVolume, PersistentVolumeClaim, Pod},
};
use kube::{
    api::{Api, DeleteParams, ListParams, ObjectMeta, Patch, PatchParams, PostParams},
    client::Client as K8sClient,
    config::{KubeConfigOptions, Kubeconfig},
    Config, Error as KubeError, ResourceExt,
};
use rand::Rng;
use serde::de::DeserializeOwned;
use std::{
    collections::{BTreeMap, HashMap},
    convert::TryFrom,
    env,
    fmt::Debug,
    fs::{self, File},
    io::Write,
    net::TcpListener,
    process::{Command, Stdio},
    str,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tempfile::TempDir;
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    time::Duration,
};

/// Gets a free port
pub fn get_free_port() -> u32 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.local_addr().unwrap().port() as u32
}

/// Dumps the given String contents into a file at the given temp directory
pub fn dump_string_to_file(
    file_name: String,
    content: String,
    tmp_dir: &TempDir,
) -> Result<String> {
    let file_path = tmp_dir.path().join(file_name.clone());
    info!("Wrote content to: {:?}", &file_path);
    let mut file = File::create(file_path).expect("Could not create file in temp dir");
    file.write_all(&content.into_bytes())
        .expect("Could not write to file");
    let file_path_str = tmp_dir.path().join(file_name).display().to_string();
    Ok(file_path_str)
}

#[derive(Error, Debug)]
#[error("{0}")]
enum LogJobError {
    RetryableError(String),
    FinalError(String),
}

/**
 * Tail the logs of a job. Returns OK if the job has a pod that succeeds.
 * Assumes that the job only runs once and exits, and has no configured retry policy (i.e. backoffLimit = 0)
 */
async fn tail_job_logs(
    jobs_api: Arc<dyn ReadWrite<Job>>,
    job_name: String,
    job_namespace: String,
) -> Result<(), LogJobError> {
    let genesis_job = jobs_api
        .get_status(&job_name)
        .await
        .map_err(|e| LogJobError::FinalError(format!("Failed to get job status: {}", e)))?;

    let status = genesis_job.status.expect("Job status not found");
    info!("Job {} status: {:?}", &job_name, status);
    match status.active {
        Some(active) => {
            if active < 1 {
                return Err(LogJobError::RetryableError(format!(
                    "Job {} has no active pods. Maybe it has not started yet",
                    &job_name
                )));
            }
            // try tailing the logs of the genesis job
            // by the time this is done, we can re-evalulate its status
            let mut command = tokio::process::Command::new(KUBECTL_BIN)
                .args([
                    "-n",
                    &job_namespace,
                    "logs",
                    "--tail=10", // in case of connection reset we only want the last few lines to avoid spam
                    "-f",
                    format!("job/{}", &job_name).as_str(),
                ])
                .stdout(Stdio::piped())
                .spawn()
                .map_err(|e| {
                    LogJobError::RetryableError(format!("Failed to spawn command: {}", e))
                })?;
            // Ensure the command has stdout
            let stdout = command.stdout.take().ok_or_else(|| {
                LogJobError::RetryableError("Failed to capture stdout".to_string())
            })?;

            // Create a BufReader to read the output asynchronously, line by line
            let mut reader = BufReader::new(stdout).lines();

            // Iterate over the lines as they come
            while let Some(line) = reader.next_line().await.transpose() {
                match line {
                    Ok(line) => {
                        info!("[{}]: {}", &job_name, line); // Add a prefix to each line
                    },
                    Err(e) => {
                        return Err(LogJobError::RetryableError(format!(
                            "Error reading line: {}",
                            e
                        )));
                    },
                }
            }
            command.wait().await.map_err(|e| {
                LogJobError::RetryableError(format!("Error waiting for command: {}", e))
            })?;
        },
        None => info!("Job {} has no active pods running", &job_name),
    }
    match status.succeeded {
        Some(_) => {
            info!("Job {} succeeded!", &job_name);
            return Ok(());
        },
        None => info!("Job {} has no succeeded pods", &job_name),
    }
    if status.failed.is_some() {
        info!("Job {} failed!", &job_name);
        return Err(LogJobError::FinalError(format!("Job {} failed", &job_name)));
    }
    Err(LogJobError::RetryableError(format!(
        "Job {} has no succeeded or failed pods. Maybe it has not started yet.",
        &job_name
    )))
}

/// Waits for a job to complete, while tailing the job's logs
pub async fn wait_log_job(
    jobs_api: Arc<dyn ReadWrite<Job>>,
    job_namespace: &str,
    job_name: String,
    retry_policy: RetryPolicy,
) -> Result<()> {
    retry_policy
        .retry_if(
            move || {
                tail_job_logs(
                    jobs_api.clone(),
                    job_name.clone(),
                    job_namespace.to_string(),
                )
            },
            |e: &LogJobError| matches!(e, LogJobError::RetryableError(_)),
        )
        .await?;
    Ok(())
}

/// Waits for a given number of HAProxy K8s Deployments to be ready
async fn wait_node_haproxy(
    kube_client: &K8sClient,
    kube_namespace: &str,
    num_haproxy: usize,
) -> Result<()> {
    aptos_retrier::retry_async(k8s_wait_nodes_strategy(), || {
        let deployments_api: Api<Deployment> = Api::namespaced(kube_client.clone(), kube_namespace);
        Box::pin(async move {
            for i in 0..num_haproxy {
                let haproxy_deployment_name =
                    format!("{}-{}-haproxy", APTOS_NODE_HELM_RELEASE_NAME, i);
                match deployments_api.get_status(&haproxy_deployment_name).await {
                    Ok(s) => {
                        let deployment_name = s.name();
                        if let Some(deployment_status) = s.status {
                            let ready_replicas = deployment_status.ready_replicas.unwrap_or(0);
                            info!(
                                "Deployment {} has {} ready_replicas",
                                deployment_name, ready_replicas
                            );
                            if ready_replicas > 0 {
                                info!("Deployment {} ready", deployment_name);
                                continue;
                            }
                        }
                        info!("Deployment {} has no status", deployment_name);
                        bail!("Deployment not ready");
                    },
                    Err(e) => {
                        info!("Failed to get deployment: {}", e);
                        bail!("Failed to get deployment: {}", e);
                    },
                }
            }
            Ok(())
        })
    })
    .await
}

/// Waits for all given K8sNodes to be ready. Called when the testnet is first started, so we may have to wait a while for
/// machines to be provisioned by the cloud provider.
async fn wait_nodes_stateful_set(
    kube_client: &K8sClient,
    kube_namespace: &str,
    nodes: &HashMap<PeerId, K8sNode>,
) -> Result<()> {
    // wait for all nodes healthy
    for node in nodes.values() {
        // retry every 10 seconds for 20 minutes
        let retry_policy = RetryPolicy::fixed(Duration::from_secs(10)).with_max_retries(120);
        wait_stateful_set(
            kube_client,
            kube_namespace,
            node.stateful_set_name(),
            1,
            retry_policy,
        )
        .await?
    }
    Ok(())
}

/// Deletes a collection of resources in k8s as part of aptos-node
async fn delete_k8s_collection<T>(
    api: Api<T>,
    name: &'static str,
    label_selector: &str,
) -> Result<()>
where
    T: kube::Resource + Clone + DeserializeOwned + Debug,
    <T as kube::Resource>::DynamicType: Default,
{
    match api
        .delete_collection(
            &DeleteParams::default(),
            &ListParams::default().labels(label_selector),
        )
        .await?
    {
        either::Left(list) => {
            let names: Vec<_> = list.iter().map(ResourceExt::name).collect();
            info!("Deleting collection of {}: {:?}", name, names);
        },
        either::Right(status) => {
            info!("Deleted collection of {}: status={:?}", name, status);
        },
    }

    Ok(())
}

/// Delete existing k8s resources in the namespace. This is essentially helm uninstall but lighter weight
pub(crate) async fn delete_k8s_resources(client: K8sClient, kube_namespace: &str) -> Result<()> {
    // selector for the helm chart
    let aptos_node_helm_selector = "app.kubernetes.io/part-of=aptos-node";
    let testnet_addons_helm_selector = "app.kubernetes.io/part-of=testnet-addons";
    let genesis_helm_selector = "app.kubernetes.io/part-of=aptos-genesis";

    // selector for manually created resources from Forge
    let forge_pfn_selector = "app.kubernetes.io/part-of=forge-pfn";

    // delete all deployments and statefulsets
    // cross this with all the compute resources created by aptos-node helm chart
    let deployments: Api<Deployment> = Api::namespaced(client.clone(), kube_namespace);
    let stateful_sets: Api<StatefulSet> = Api::namespaced(client.clone(), kube_namespace);
    let pvcs: Api<PersistentVolumeClaim> = Api::namespaced(client.clone(), kube_namespace);
    let jobs: Api<Job> = Api::namespaced(client.clone(), kube_namespace);
    // service deletion by label selector is not supported in this version of k8s api
    // let services: Api<Service> = Api::namespaced(client.clone(), kube_namespace);

    for selector in &[
        aptos_node_helm_selector,
        testnet_addons_helm_selector,
        genesis_helm_selector,
        forge_pfn_selector,
    ] {
        info!("Deleting k8s resources with selector: {}", selector);
        delete_k8s_collection(deployments.clone(), "Deployments", selector).await?;
        delete_k8s_collection(stateful_sets.clone(), "StatefulSets", selector).await?;
        delete_k8s_collection(pvcs.clone(), "PersistentVolumeClaims", selector).await?;
        delete_k8s_collection(jobs.clone(), "Jobs", selector).await?;
        // This is causing problem on gcp forge for some reason?!
        // HACK remove to unblock
        // delete_k8s_collection(cronjobs.clone(), "CronJobs", selector).await?;
        // delete_k8s_collection(services.clone(), "Services", selector).await?;
    }

    delete_all_chaos(kube_namespace)?;

    Ok(())
}

pub(crate) fn delete_all_chaos(kube_namespace: &str) -> Result<()> {
    // clear everything manually, in case there are some dangling
    let delete_networkchaos = ["-n", kube_namespace, "delete", "networkchaos", "--all"];
    info!("{:?}", delete_networkchaos);
    let delete_networkchaos_output = Command::new(KUBECTL_BIN)
        .stdout(Stdio::inherit())
        .args(delete_networkchaos)
        .output()
        .expect("failed to delete all NetworkChaos");
    if !delete_networkchaos_output.status.success() {
        bail!(
            "{}",
            String::from_utf8(delete_networkchaos_output.stderr).unwrap()
        );
    }
    Ok(())
}

/// Deletes all Forge resources from the given namespace. If the namespace is "default", delete the management configmap
/// as well as all compute resources. If the namespace is a Forge namespace (has the "forge-*" prefix), then simply delete
/// the entire namespace
async fn delete_k8s_cluster(kube_namespace: String) -> Result<()> {
    let client: K8sClient = create_k8s_client().await?;

    // if operating on the default namespace,
    match kube_namespace.as_str() {
        "default" => {
            // delete the management configmap
            let configmap: Api<ConfigMap> = Api::namespaced(client.clone(), &kube_namespace);
            let management_configmap_name =
                format!("{}-{}", MANAGEMENT_CONFIGMAP_PREFIX, &kube_namespace);
            match configmap
                .delete(&management_configmap_name, &DeleteParams::default())
                .await
            {
                Ok(_) => info!(
                    "Deleted default management configmap: {}",
                    &management_configmap_name
                ),
                // if configmap not found, assume it's already been deleted and make clean-up idempotent
                Err(KubeError::Api(api_err)) => {
                    if api_err.code == 404 {
                        info!(
                            "Could not find configmap {}, continuing",
                            &management_configmap_name
                        );
                    } else {
                        bail!(api_err);
                    }
                },
                Err(e) => bail!(e),
            };
            delete_k8s_resources(client, "default").await?;
        },
        s if s.starts_with("forge") => {
            let namespaces: Api<Namespace> = Api::all(client);
            namespaces
                .delete(&kube_namespace, &DeleteParams::default())
                .await?
                .map_left(|namespace| info!("Deleting namespace {}: {:?}", s, namespace.status))
                .map_right(|status| info!("Deleted namespace {}: {:?}", s, status));
        },
        _ => {
            bail!(
                "Invalid kubernetes namespace provided: {}. Use forge-*",
                kube_namespace
            );
        },
    }

    Ok(())
}

pub async fn uninstall_testnet_resources(kube_namespace: String) -> Result<()> {
    // delete kubernetes resources
    delete_k8s_cluster(kube_namespace.clone()).await?;
    info!(
        "aptos-node resources for Forge removed in namespace: {}",
        kube_namespace
    );

    Ok(())
}

pub fn generate_new_era() -> String {
    let mut rng = rand::thread_rng();
    let r: u8 = rng.gen();
    format!("forge{}", r)
}

fn get_node_default_helm_path() -> String {
    match ForgeRunnerMode::try_from_env().unwrap_or(ForgeRunnerMode::K8s) {
        ForgeRunnerMode::Local => {
            "testsuite/forge/src/backend/k8s/helm-values/aptos-node-default-values.yaml"
        },
        ForgeRunnerMode::K8s => "/aptos/terraform/aptos-node-default-values.yaml",
    }
    .to_string()
}

pub async fn reset_persistent_volumes(kube_client: &K8sClient) -> Result<()> {
    let pv_api: Api<PersistentVolume> = Api::all(kube_client.clone());
    let pvs = pv_api
        .list(&ListParams::default())
        .await?
        .items
        .into_iter()
        .filter(|pv| {
            if let Some(status) = &pv.status {
                if let Some(phase) = &status.phase {
                    if phase == "Released" {
                        return true;
                    }
                }
            }
            false
        })
        .collect::<Vec<PersistentVolume>>();

    for pv in &pvs {
        let name = pv.metadata.name.clone().expect("Must have name!");
        info!("Changing pv {} from Released to Available.", name);
        let patch = serde_json::json!({
            "spec": {
                "claimRef": null
            }
        });
        pv_api
            .patch(&name, &PatchParams::default(), &Patch::Merge(&patch))
            .await?;
    }

    Ok(())
}

pub async fn check_persistent_volumes(
    kube_client: K8sClient,
    num_requested_pvs: usize,
    existing_db_tag: String,
) -> Result<()> {
    info!("Trying to get {} PVs.", num_requested_pvs);
    let pv_api: Api<PersistentVolume> = Api::all(kube_client.clone());
    let list_params = ListParams::default();
    let pvs = pv_api
        .list(&list_params)
        .await?
        .items
        .into_iter()
        .filter(|pv| {
            if let Some(labels) = &pv.metadata.labels {
                if let Some(tag) = labels.get(&"tag".to_string()) {
                    if tag == &existing_db_tag {
                        if let Some(status) = &pv.status {
                            if let Some(phase) = &status.phase {
                                if phase == "Available" {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
            false
        })
        .collect::<Vec<PersistentVolume>>();

    if pvs.len() < num_requested_pvs {
        return Err(anyhow!(
            "Could not find enough PVs, requested: {}, available: {}.",
            num_requested_pvs,
            pvs.len()
        ));
    }

    info!("Found enough PVs.");

    Ok(())
}

/// Get the existing helm values for a release
fn get_default_helm_release_values_from_cluster(
    helm_release_name: &str,
) -> Result<serde_yaml::Value> {
    let status_args = [
        "status",
        helm_release_name,
        "--namespace",
        "default",
        "-o",
        "yaml",
    ];
    info!("{:?}", status_args);
    let raw_helm_values = Command::new(HELM_BIN)
        .args(status_args)
        .output()
        .unwrap_or_else(|_| panic!("Failed to helm status {}", helm_release_name));

    let helm_values = String::from_utf8(raw_helm_values.stdout).unwrap();
    let j: serde_yaml::Value = serde_yaml::from_str(&helm_values).map_err(|e| {
        format_err!(
            "Failed to deserialize helm values. Check if release {} exists: {}",
            helm_release_name,
            e
        )
    })?;
    // get .config or anyhow bail!
    let config = j
        .get("config")
        .ok_or_else(|| anyhow!("Failed to get helm values"))?;
    Ok(config.clone())
}

/// Merges two YAML values in place, with `b` taking precedence over `a`
/// This simulates helm's behavior of merging default values (values.yaml) with overridden values specified (-f file or --set)
/// Source: https://stackoverflow.com/questions/67727239/how-to-combine-including-nested-array-values-two-serde-yamlvalue-objects
fn merge_yaml(a: &mut serde_yaml::Value, b: serde_yaml::Value) {
    match (a, b) {
        (a @ &mut serde_yaml::Value::Mapping(_), serde_yaml::Value::Mapping(b)) => {
            let a = a.as_mapping_mut().unwrap();
            for (k, v) in b {
                if v.is_sequence() && a.contains_key(&k) && a[&k].is_sequence() {
                    let mut _b = a.get(&k).unwrap().as_sequence().unwrap().to_owned();
                    _b.append(&mut v.as_sequence().unwrap().to_owned());
                    a[&k] = serde_yaml::Value::from(_b);
                    continue;
                }
                if !a.contains_key(&k) {
                    a.insert(k.to_owned(), v.to_owned());
                } else {
                    merge_yaml(&mut a[&k], v);
                }
            }
        },
        (a, b) => *a = b,
    }
}

/// Installs a testnet in a k8s namespace by first running genesis, and the installing the aptos-nodes via helm
/// Returns all validators and fullnodes by collecting the running nodes
pub async fn install_testnet_resources(
    new_era: String,
    kube_namespace: String,
    num_validators: usize,
    num_fullnodes: usize,
    node_image_tag: String,
    genesis_image_tag: String,
    genesis_modules_path: Option<String>,
    use_port_forward: bool,
    enable_haproxy: bool,
    enable_indexer: bool,
    deployer_profile: String,
    genesis_helm_config_fn: Option<GenesisConfigFn>,
    node_helm_config_fn: Option<NodeConfigFn>,
    // If true, skip collecting running nodes after installing the testnet. This is useful when we only care about creating resources
    // but not healthchecking or collecting the nodes for further operations. Setting this to "true" effectively makes the return type useless though.
    skip_collecting_running_nodes: bool,
) -> Result<(HashMap<PeerId, K8sNode>, HashMap<PeerId, K8sNode>)> {
    let kube_client = create_k8s_client().await?;

    // get existing helm values from the cluster
    // if the release doesn't exist, return an empty mapping, which may work, especially as we move away from this pattern and instead having default values baked into the deployer
    let mut aptos_node_helm_values =
        get_default_helm_release_values_from_cluster(APTOS_NODE_HELM_RELEASE_NAME)
            .unwrap_or_else(|_| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));
    let mut genesis_helm_values =
        get_default_helm_release_values_from_cluster(GENESIS_HELM_RELEASE_NAME)
            .unwrap_or_else(|_| serde_yaml::Value::Mapping(serde_yaml::Mapping::new()));

    let aptos_node_helm_values_override = construct_node_helm_values_from_input(
        node_helm_config_fn,
        fs::read_to_string(get_node_default_helm_path())
            .expect("Not able to read default value file"),
        kube_namespace.clone(),
        new_era.clone(),
        num_validators,
        num_fullnodes,
        node_image_tag,
        enable_haproxy,
    )?;

    let genesis_helm_values_override = construct_genesis_helm_values_from_input(
        genesis_helm_config_fn,
        kube_namespace.clone(),
        new_era.clone(),
        num_validators,
        genesis_image_tag,
        enable_haproxy,
    )?;

    info!("aptos_node_helm_values: {:?}", aptos_node_helm_values);
    info!(
        "aptos_node_helm_values_override: {:?}",
        aptos_node_helm_values_override
    );

    merge_yaml(&mut aptos_node_helm_values, aptos_node_helm_values_override);
    merge_yaml(&mut genesis_helm_values, genesis_helm_values_override);

    info!(
        "aptos_node_helm_values after override: {:?}",
        aptos_node_helm_values
    );

    // disable uploading genesis to blob storage since indexer requires it in the cluster
    if enable_indexer {
        aptos_node_helm_values["genesis_blob_upload_url"] = "".into();
    }
    // run genesis from this directory in the image
    if let Some(genesis_modules_path) = genesis_modules_path {
        genesis_helm_values["genesis"]["moveModulesDir"] = genesis_modules_path.into();
    }
    // disable uploading genesis to blob storage since indexer requires it in the cluster
    if enable_indexer {
        genesis_helm_values["genesis"]["genesis_blob_upload_url"] = "".into();
    }

    let config: serde_json::Value = serde_json::from_value(serde_json::json!({
        "profile": deployer_profile,
        "era": new_era,
        "namespace": kube_namespace.clone(),
        "testnet-values": aptos_node_helm_values,
        "genesis-values": genesis_helm_values,
    }))?;

    let testnet_deployer = ForgeDeployerManager::new(
        kube_client.clone(),
        kube_namespace.clone(),
        FORGE_TESTNET_DEPLOYER_DOCKER_IMAGE_REPO.to_string(),
        // Some("423433fe2b3e1a814040b8f981364e7d4368519b".to_string()),
        // Some("c2a0de5f1fff4bb301d1b1841d27037e5173177c".to_string()),
        Some("c1ca04bcd09c3b207c77d67bc2bf908b296245ac".to_string()),
    );

    testnet_deployer.start(config).await?;
    testnet_deployer.wait_completed().await?;

    if skip_collecting_running_nodes {
        Ok((HashMap::new(), HashMap::new()))
    } else {
        let (validators, fullnodes) = collect_running_nodes(
            &kube_client,
            kube_namespace,
            use_port_forward,
            enable_haproxy,
        )
        .await?;
        Ok((validators, fullnodes))
    }
}

pub fn construct_node_helm_values_from_input(
    node_helm_config_fn: Option<NodeConfigFn>,
    base_helm_values: String,
    kube_namespace: String,
    era: String,
    num_validators: usize,
    num_fullnodes: usize,
    image_tag: String,
    enable_haproxy: bool,
) -> Result<serde_yaml::Value> {
    let mut value: serde_yaml::Value = serde_yaml::from_str(&base_helm_values)?;
    value["numValidators"] = num_validators.into();
    value["numFullnodeGroups"] = num_fullnodes.into();
    value["imageTag"] = image_tag.clone().into();
    value["chain"]["era"] = era.into();
    value["haproxy"]["enabled"] = enable_haproxy.into();
    value["labels"]["forge-namespace"] = make_k8s_label(kube_namespace).into();
    value["labels"]["forge-image-tag"] = make_k8s_label(image_tag).into();

    // if present, tag the node with the test suite name and username
    let suite_name = env::var("FORGE_TEST_SUITE").unwrap_or(DEFAULT_TEST_SUITE_NAME.to_string());
    value["labels"]["forge-test-suite"] = make_k8s_label(suite_name).into();
    let username = env::var("FORGE_USERNAME").unwrap_or(DEFAULT_USERNAME.to_string());
    value["labels"]["forge-username"] = make_k8s_label(username).into();

    if let Some(config_fn) = node_helm_config_fn {
        (config_fn)(&mut value);
    }
    Ok(value)
}

pub fn construct_genesis_helm_values_from_input(
    genesis_helm_config_fn: Option<GenesisConfigFn>,
    kube_namespace: String,
    era: String,
    num_validators: usize,
    genesis_image_tag: String,
    enable_haproxy: bool,
) -> Result<serde_yaml::Value> {
    let validator_internal_host_suffix = if enable_haproxy {
        VALIDATOR_HAPROXY_SERVICE_SUFFIX
    } else {
        VALIDATOR_SERVICE_SUFFIX
    };
    let fullnode_internal_host_suffix = if enable_haproxy {
        FULLNODE_HAPROXY_SERVICE_SUFFIX
    } else {
        FULLNODE_SERVICE_SUFFIX
    };
    let mut value: serde_yaml::Value = serde_yaml::Value::default();
    value["imageTag"] = genesis_image_tag.clone().into();
    value["chain"]["era"] = era.into();
    value["chain"]["root_key"] = DEFAULT_ROOT_KEY.into();
    value["genesis"]["numValidators"] = num_validators.into();
    value["genesis"]["validator"]["internal_host_suffix"] = validator_internal_host_suffix.into();
    value["genesis"]["validator"]["key_seed"] = FORGE_KEY_SEED.into();
    value["genesis"]["fullnode"]["internal_host_suffix"] = fullnode_internal_host_suffix.into();
    value["labels"]["forge-namespace"] = make_k8s_label(kube_namespace).into();
    value["labels"]["forge-image-tag"] = make_k8s_label(genesis_image_tag).into();

    // if present, tag the node with the test suite name and username
    let suite_name = env::var("FORGE_TEST_SUITE").unwrap_or(DEFAULT_TEST_SUITE_NAME.to_string());
    value["labels"]["forge-test-suite"] = make_k8s_label(suite_name).into();
    let username = env::var("FORGE_USERNAME").unwrap_or(DEFAULT_USERNAME.to_string());
    value["labels"]["forge-username"] = make_k8s_label(username).into();

    if let Some(config_fn) = genesis_helm_config_fn {
        (config_fn)(&mut value);
    }

    Ok(value)
}

/// Collect the running nodes in the network into K8sNodes
pub async fn collect_running_nodes(
    kube_client: &K8sClient,
    kube_namespace: String,
    use_port_forward: bool,
    enable_haproxy: bool,
) -> Result<(HashMap<PeerId, K8sNode>, HashMap<PeerId, K8sNode>)> {
    // get all validators
    let validators = get_validators(
        kube_client.clone(),
        &kube_namespace,
        use_port_forward,
        enable_haproxy,
    )
    .await
    .unwrap();

    // wait for all validator STS to spin up
    wait_nodes_stateful_set(kube_client, &kube_namespace, &validators).await?;

    if enable_haproxy {
        wait_node_haproxy(kube_client, &kube_namespace, validators.len()).await?;
    }

    // get all fullnodes
    let validator_fullnodes = get_validator_fullnodes(
        kube_client.clone(),
        &kube_namespace,
        use_port_forward,
        enable_haproxy,
    )
    .await
    .unwrap();

    wait_nodes_stateful_set(kube_client, &kube_namespace, &validator_fullnodes).await?;

    let nodes = validators
        .values()
        .chain(validator_fullnodes.values())
        .collect::<Vec<&K8sNode>>();

    // start port-forward for each of the nodes
    if use_port_forward {
        for node in nodes.iter() {
            node.port_forward_rest_api()?;
            // assume this will always succeed???
        }
    }

    // nodes_healthcheck(nodes).await?;
    Ok((validators, validator_fullnodes))
}

/// Returns a [Config] object reading the KUBECONFIG environment variable or infering from the
/// environment. Differently from [`Config::infer()`], this will look at the
/// `KUBECONFIG` env var first, and only then infer from the environment.
async fn make_kube_client_config() -> Result<Config> {
    match Config::from_kubeconfig(&KubeConfigOptions::default()).await {
        Ok(config) => Ok(config),
        Err(kubeconfig_err) => {
            Config::infer()
                .await
                .map_err(|infer_err|
                    anyhow::anyhow!("Unable to construct Config. Failed to infer config {:?}. Failed to read KUBECONFIG {:?}", infer_err, kubeconfig_err)
                )
        }
    }
}

pub async fn create_k8s_client() -> Result<K8sClient> {
    let mut config = make_kube_client_config().await?;

    let cluster_name = Kubeconfig::read()
        .map(|k| k.current_context.unwrap_or_default())
        .unwrap_or_else(|_| config.cluster_url.to_string());

    config.accept_invalid_certs = true;

    let client = K8sClient::try_from(config)?;

    // Test the connection, fail if request fails
    client.apiserver_version().await.map_err(|_| {
        if !cluster_name.contains("forge") {
            format_err!(
                "Failed to connect to kubernetes cluster {}, \
                please make sure you have the right kubeconfig",
                cluster_name
            )
        } else {
            format_err!("Failed to connect to kubernetes cluster {}", cluster_name)
        }
    })?;
    Ok(client)
}

#[derive(Error, Debug)]
#[error("{0}")]
pub enum ApiError {
    RetryableError(String),
    FinalError(String),
}

/// Does the same as create_namespace and handling the 409, but for any k8s resource T
pub async fn maybe_create_k8s_resource<T>(
    api: Arc<dyn ReadWrite<T>>,
    resource: T,
) -> Result<T, ApiError>
where
    T: kube::Resource + Clone + DeserializeOwned + Debug,
    <T as kube::Resource>::DynamicType: Default,
{
    if let Err(KubeError::Api(api_err)) = api.create(&PostParams::default(), &resource).await {
        if api_err.code == 409 {
            info!(
                "Resource {:?}, {} already exists, continuing with it",
                std::any::type_name::<T>(),
                resource.name()
            );
        } else {
            return Err(ApiError::RetryableError(format!(
                "Failed to use existing resource{:?} {}: {:?}",
                std::any::type_name::<T>(),
                resource.name(),
                api_err
            )));
        }
    }
    Ok(resource)
}

pub async fn create_namespace(
    namespace_api: Arc<dyn ReadWrite<Namespace>>,
    kube_namespace: String,
) -> Result<Namespace, ApiError> {
    let kube_namespace_name = kube_namespace.clone();
    let namespace = Namespace {
        metadata: ObjectMeta {
            name: Some(kube_namespace_name.clone()),
            ..ObjectMeta::default()
        },
        spec: None,
        status: None,
    };
    if let Err(KubeError::Api(api_err)) = namespace_api
        .create(&PostParams::default(), &namespace)
        .await
    {
        if api_err.code == 409 {
            info!(
                "Namespace {} already exists, continuing with it",
                &kube_namespace_name
            );
        } else if api_err.code == 401 {
            return Err(ApiError::FinalError(
                "Unauthorized, did you authorize with kubernetes? \
                    Try running `kubectl get current-context`"
                    .to_string(),
            ));
        } else {
            return Err(ApiError::RetryableError(format!(
                "Failed to use existing namespace {}: {:?}",
                &kube_namespace_name, api_err
            )));
        }
    }
    Ok(namespace)
}

pub async fn create_management_configmap(
    kube_namespace: String,
    keep: bool,
    cleanup_duration: Duration,
) -> Result<()> {
    let kube_client = create_k8s_client().await?;
    let namespaces_api = Arc::new(K8sApi::<Namespace>::from_client(kube_client.clone(), None));
    let other_kube_namespace = kube_namespace.clone();

    // try to create a new namespace
    // * if it errors with 409, the namespace exists already and we should use it
    // * if it errors with 403, the namespace is likely in the process of being terminated, so try again
    RetryPolicy::exponential(Duration::from_millis(1000))
        .with_max_delay(Duration::from_millis(10 * 60 * 1000))
        .retry_if(
            move || create_namespace(namespaces_api.clone(), other_kube_namespace.clone()),
            |e: &ApiError| matches!(e, ApiError::RetryableError(_)),
        )
        .await?;

    let configmap_api = Arc::new(K8sApi::<ConfigMap>::from_client(
        kube_client.clone(),
        Some(kube_namespace.clone()),
    ));

    let management_configmap_name = format!("{}-{}", MANAGEMENT_CONFIGMAP_PREFIX, &kube_namespace);
    let mut data: BTreeMap<String, String> = BTreeMap::new();
    let start = SystemTime::now();
    let cleanup_time = (start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        + cleanup_duration)
        .as_secs();
    data.insert("keep".to_string(), keep.to_string());
    data.insert("cleanup".to_string(), cleanup_time.to_string());

    let config = ConfigMap {
        binary_data: None,
        data: Some(data.clone()),
        metadata: ObjectMeta {
            name: Some(management_configmap_name.clone()),
            ..ObjectMeta::default()
        },
        immutable: None,
    };
    if let Err(KubeError::Api(api_err)) =
        configmap_api.create(&PostParams::default(), &config).await
    {
        if api_err.code == 409 {
            info!(
                "Configmap {} already exists, continuing with it",
                &management_configmap_name
            );
        } else {
            bail!(
                "Failed to use existing management configmap {}: {:?}",
                &kube_namespace,
                api_err
            );
        }
    } else {
        info!(
            "Created configmap {} with data {:?}",
            management_configmap_name, data
        );
    }

    Ok(())
}

pub async fn cleanup_cluster_with_management() -> Result<()> {
    let kube_client = create_k8s_client().await?;
    let start = SystemTime::now();
    let time_since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();

    let pods_api: Api<Pod> = Api::namespaced(kube_client.clone(), "default");
    let lp = ListParams::default().labels("app.kubernetes.io/name=forge");

    // delete all forge test pods over a threshold age
    let pods = pods_api
        .list(&lp)
        .await?
        .items
        .into_iter()
        .filter(|pod| {
            let pod_name = pod.name();
            info!("Got pod {}", pod_name);
            if let Some(time) = &pod.metadata.creation_timestamp {
                let pod_creation_time = time.0.timestamp() as u64;
                let pod_uptime = time_since_the_epoch - pod_creation_time;
                info!(
                    "Pod {} has lived for {}/{} seconds",
                    pod_name, pod_uptime, POD_CLEANUP_THRESHOLD_SECS
                );
                if pod_uptime > POD_CLEANUP_THRESHOLD_SECS {
                    return true;
                }
            }
            false
        })
        .collect::<Vec<Pod>>();
    for pod in pods {
        let pod_name = pod.name();
        info!("Deleting pod {}", pod_name);
        pods_api.delete(&pod_name, &DeleteParams::default()).await?;
    }

    // delete all forge testnets over a threshold age using their management configmaps
    // unless they are explicitly set with "keep = true"
    let configmaps_api: Api<ConfigMap> = Api::all(kube_client.clone());
    let lp = ListParams::default();
    let configmaps = configmaps_api
        .list(&lp)
        .await?
        .items
        .into_iter()
        .filter(|configmap| {
            let configmap_name = configmap.name();
            let configmap_namespace = configmap.namespace().unwrap();
            if !configmap_name.contains(MANAGEMENT_CONFIGMAP_PREFIX) {
                return false;
            }
            if let Some(data) = &configmap.data {
                info!("Got configmap {} with data: {:?}", &configmap_name, data);
                return check_namespace_for_cleanup(
                    data,
                    configmap_namespace,
                    time_since_the_epoch,
                );
            }
            false
        })
        .collect::<Vec<ConfigMap>>();
    for configmap in configmaps {
        let namespace = configmap.namespace().unwrap();
        uninstall_testnet_resources(namespace).await?;
    }

    Ok(())
}

fn check_namespace_for_cleanup(
    data: &BTreeMap<String, String>,
    namespace: String,
    time_since_the_epoch: u64,
) -> bool {
    let keep: bool = data.get("keep").unwrap().parse().unwrap();
    if keep {
        info!("Explicitly keeping namespace {}", namespace);
        return false;
    }
    if data.get("cleanup").is_none() {
        // This is needed for backward compatibility where older namespaces created
        // don't have "cleanup" time set. Delete this code once we roll out the cleanup
        // feature fully
        let start: u64 = data.get("start").unwrap().parse().unwrap();
        let namespace_uptime = time_since_the_epoch - start;
        info!(
            "Namespace {} has lived for {}/{} seconds",
            namespace, namespace_uptime, NAMESPACE_CLEANUP_THRESHOLD_SECS
        );
        if keep {
            info!("Explicitly keeping namespace {}", namespace);
            return false;
        }
        if namespace_uptime > NAMESPACE_CLEANUP_THRESHOLD_SECS {
            return true;
        }
    } else {
        // TODO(rustielin): come up with some sane values for namespaces
        let cleanup_time_since_epoch: u64 = data.get("cleanup").unwrap().parse().unwrap();

        if cleanup_time_since_epoch <= time_since_the_epoch {
            info!("Namespace {} will be cleaned up", namespace,);
            return true;
        } else {
            info!(
                "Namespace {} has remaining {} seconds before cleanup",
                namespace,
                cleanup_time_since_epoch - time_since_the_epoch
            );
        }
    }
    false
}

/// Ensures that the label is at most 64 characters to meet k8s
/// label length requirements.
pub fn make_k8s_label(value: String) -> String {
    value.get(..63).unwrap_or(&value).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FailedK8sResourceApi;

    #[tokio::test]
    async fn test_create_namespace_final_error() {
        let namespace_creator = Arc::new(FailedK8sResourceApi::from_status_code(401));
        let result = create_namespace(namespace_creator, "banana".to_string()).await;
        match result {
            Err(ApiError::FinalError(_)) => {},
            _ => panic!("Expected final error"),
        }
    }

    #[tokio::test]
    async fn test_construct_node_helm_values() {
        let node_helm_values = construct_node_helm_values_from_input(
            None,
            "{}".to_string(),
            "forge-123".to_string(),
            "era".to_string(),
            5,
            6,
            "image".to_string(),
            true,
        )
        .unwrap();

        let node_helm_values_str = serde_yaml::to_string(&node_helm_values).unwrap();

        let expected_helm_values = "---
numValidators: 5
numFullnodeGroups: 6
imageTag: image
chain:
  era: era
haproxy:
  enabled: true
labels:
  forge-namespace: forge-123
  forge-image-tag: image
  forge-test-suite: unknown-testsuite
  forge-username: unknown-username
";
        assert_eq!(node_helm_values_str, expected_helm_values);
    }

    #[tokio::test]
    async fn test_construct_genesis_helm_values() {
        let genesis_helm_values = construct_genesis_helm_values_from_input(
            Some(Arc::new(|helm_values| {
                helm_values["chain"]["epoch_duration_secs"] = 60.into();
            })),
            "forge-123".to_string(),
            "era".to_string(),
            5,
            "genesis_image".to_string(),
            true,
        )
        .unwrap();
        let genesis_helm_values_str = serde_yaml::to_string(&genesis_helm_values).unwrap();
        let expected_helm_values = "---
imageTag: genesis_image
chain:
  era: era
  root_key: 48136DF3174A3DE92AFDB375FFE116908B69FF6FAB9B1410E548A33FEA1D159D
  epoch_duration_secs: 60
genesis:
  numValidators: 5
  validator:
    internal_host_suffix: validator-lb
    key_seed: \"80000\"
  fullnode:
    internal_host_suffix: fullnode-lb
labels:
  forge-namespace: forge-123
  forge-image-tag: genesis_image
  forge-test-suite: unknown-testsuite
  forge-username: unknown-username
";
        assert_eq!(genesis_helm_values_str, expected_helm_values);
        println!("{}", genesis_helm_values_str);
    }

    #[tokio::test]
    async fn test_create_namespace_retryable_error() {
        let namespace_creator = Arc::new(FailedK8sResourceApi::from_status_code(403));
        let result = create_namespace(namespace_creator, "banana".to_string()).await;
        match result {
            Err(ApiError::RetryableError(_)) => {},
            _ => panic!("Expected retryable error"),
        }
    }

    #[tokio::test]
    async fn test_check_namespace_for_cleanup() {
        let start = SystemTime::now();
        let time_since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let mut data = BTreeMap::new();

        // Ensure very old run without keep is cleaned up.
        data.insert("keep".to_string(), "false".to_string());
        data.insert("start".to_string(), "0".to_string());

        assert!(check_namespace_for_cleanup(
            &data,
            "foo".to_string(),
            time_since_the_epoch
        ));

        // Ensure old run with keep is not cleaned up.
        data.insert("keep".to_string(), "true".to_string());
        data.insert("start".to_string(), "0".to_string());

        assert!(!check_namespace_for_cleanup(
            &data,
            "foo".to_string(),
            time_since_the_epoch
        ));

        // Ensure very old run without keep is cleaned up.
        data.insert("keep".to_string(), "false".to_string());
        data.insert("cleanup".to_string(), "20".to_string());

        assert!(check_namespace_for_cleanup(
            &data,
            "foo".to_string(),
            time_since_the_epoch
        ));

        // Ensure old run with keep is not cleaned up.
        data.insert("keep".to_string(), "true".to_string());
        data.insert("cleanup".to_string(), "20".to_string());

        assert!(!check_namespace_for_cleanup(
            &data,
            "foo".to_string(),
            time_since_the_epoch
        ));

        // Ensure a run with clean up some time in future is not cleaned up.
        data.insert("keep".to_string(), "false".to_string());
        let cleanup_time = (start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            + Duration::from_secs(3600))
        .as_secs();
        data.insert("cleanup".to_string(), cleanup_time.to_string());

        assert!(!check_namespace_for_cleanup(
            &data,
            "foo".to_string(),
            time_since_the_epoch
        ));
    }

    #[test]
    fn test_merge_yaml_values() {
        let yaml1 = r#"
        foo:
          bar: 1
          baz:
            qux: hello
        "#;

        let yaml2 = r#"
        foo:
          bar: 2
          baz:
            quux: world
        extra: something
        "#;

        let mut value1: serde_yaml::Value = serde_yaml::from_str(yaml1).unwrap();
        let value2: serde_yaml::Value = serde_yaml::from_str(yaml2).unwrap();

        let merged_with_serde_merge_tmerge: serde_yaml::Value =
            serde_merge::tmerge(&mut value1, &value2).unwrap();
        merge_yaml(&mut value1, value2); // this is an in-place merge
        let merged_with_crate = value1;

        let expected: serde_yaml::Value = serde_yaml::from_str(
            r#"
        foo:
          bar: 2
          baz:
            qux: hello
            quux: world
        extra: something
        "#,
        )
        .unwrap();

        assert_ne!(merged_with_serde_merge_tmerge, expected);
        assert_eq!(merged_with_crate, expected);
    }
}
