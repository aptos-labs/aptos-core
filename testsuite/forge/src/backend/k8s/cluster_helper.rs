// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    get_validators, k8s_wait_genesis_strategy, nodes_healthcheck, Result, DEFAULT_ROOT_KEY,
};
use anyhow::{bail, format_err};
use k8s_openapi::api::{
    apps::v1::{Deployment, StatefulSet},
    batch::v1::Job,
    core::v1::Namespace,
};
use kube::{
    api::{Api, DeleteParams, ListParams, Meta},
    client::Client as K8sClient,
    Config,
};
use rand::Rng;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{
    convert::TryFrom,
    fs::File,
    io::Write,
    process::{Command, Stdio},
    str,
};
use tempfile::TempDir;

const HELM_BIN: &str = "helm";
const KUBECTL_BIN: &str = "kubectl";
const MAX_NUM_VALIDATORS: usize = 30;
const APTOS_NODE_HELM_RELEASE_NAME: &str = "aptos-node";
const GENESIS_HELM_RELEASE_NAME: &str = "genesis";

async fn wait_genesis_job(kube_client: &K8sClient, era: &str, kube_namespace: &str) -> Result<()> {
    aptos_retrier::retry_async(k8s_wait_genesis_strategy(), || {
        let jobs: Api<Job> = Api::namespaced(kube_client.clone(), kube_namespace);
        Box::pin(async move {
            let job_name = format!("{}-aptos-genesis-e{}", GENESIS_HELM_RELEASE_NAME, era);
            println!("Checking status of k8s job: {}", &job_name);
            let genesis_job = jobs.get_status(&job_name).await.unwrap();
            println!("Status: {:?}", genesis_job.status);

            let status = genesis_job.status.unwrap();
            match status.active {
                Some(_) => bail!("Genesis still running or pending"),
                None => println!("Genesis completed running"),
            }
            match status.succeeded {
                Some(_) => {
                    println!("Genesis done");
                    Ok(())
                }
                _ => bail!("Genesis did not succeed"),
            }
        })
    })
    .await
}

pub fn set_validator_image_tag(
    validator_name: String,
    image_tag: String,
    helm_repo: String,
) -> Result<()> {
    let validator_upgrade_options = vec![
        "--reuse-values".to_string(),
        "--history-max".to_string(),
        "2".to_string(),
        "--set".to_string(),
        format!("imageTag={}", image_tag),
    ];
    upgrade_validator(validator_name, helm_repo, &validator_upgrade_options)
}

/// Deletes a collection of resources in k8s as part of aptos-node
async fn delete_k8s_collection<T: Clone + DeserializeOwned + Meta>(
    api: Api<T>,
    name: &'static str,
) -> Result<()> {
    match api
        .delete_collection(
            &DeleteParams::default(),
            &ListParams::default().labels("app.kubernetes.io/part-of=aptos-node"),
        )
        .await?
    {
        either::Left(list) => {
            let names: Vec<_> = list.iter().map(Meta::name).collect();
            println!("Deleting collection of {}: {:?}", name, names);
        }
        either::Right(status) => {
            println!("Deleted collection of {}: status={:?}", name, status);
        }
    }

    Ok(())
}

pub(crate) async fn delete_k8s_cluster(kube_namespace: String) -> Result<()> {
    let client: K8sClient = create_k8s_client().await;

    // if operating on the default namespace,
    match kube_namespace.as_str() {
        "default" => {
            let deployments: Api<Deployment> = Api::namespaced(client.clone(), "default");
            let stateful_sets: Api<StatefulSet> = Api::namespaced(client, "default");

            // delete all deployments and statefulsets
            // cross this with all the compute resources created by aptos-node helm chart
            delete_k8s_collection(deployments, "deployments").await?;
            delete_k8s_collection(stateful_sets, "stateful_sets").await?;
        }
        s if s.starts_with("forge") => {
            let namespaces: Api<Namespace> = Api::all(client);
            namespaces
                .delete(&kube_namespace, &DeleteParams::default())
                .await?
                .map_left(|o| println!("Deleting namespace: {:?}", o.status))
                .map_right(|s| println!("Deleted namespace: {:?}", s));
        }
        _ => {
            bail!("Invalid KUBE_NAMESPACE name");
        }
    }

    Ok(())
}

fn upgrade_helm_release(
    release_name: String,
    helm_chart: String,
    options: &[String],
    kube_namespace: String,
) -> Result<()> {
    let upgrade_base_args = [
        "upgrade".to_string(),
        "--install".to_string(),
        // forge
        "--force".to_string(),
        // in a new namespace
        "--create-namespace".to_string(),
        "--namespace".to_string(),
        kube_namespace,
        // upgrade
        release_name.clone(),
        helm_chart.clone(),
        // reuse old values
        "--reuse-values".to_string(),
        "--history-max".to_string(),
        "2".to_string(),
        // do not create cluster-level resources
        "--set".to_string(),
        "podSecurityPolicy=false".to_string(),
    ];
    let upgrade_args = [&upgrade_base_args, options].concat();
    println!("{:?}", upgrade_args);
    let upgrade_output = Command::new(HELM_BIN)
        .stdout(Stdio::inherit())
        .args(&upgrade_args)
        .output()
        .unwrap_or_else(|_| {
            panic!(
                "failed to helm upgrade release {} with chart {}",
                release_name, helm_chart
            )
        });
    if !upgrade_output.status.success() {
        bail!(format!(
            "Upgrade not completed: {}",
            String::from_utf8(upgrade_output.stderr).unwrap()
        ));
    }

    Ok(())
}

fn upgrade_validator(validator_name: String, helm_repo: String, options: &[String]) -> Result<()> {
    // upgrade_helm_release(
    //     validator_name,
    //     format!("{}/aptos-validator", helm_repo),
    //     options,
    // )
    Ok(())
}

fn upgrade_aptos_node_helm(
    release_name: String,
    helm_repo: &str,
    options: &[String],
    kube_namespace: String,
) -> Result<()> {
    upgrade_helm_release(
        release_name,
        format!("{}/aptos-node", helm_repo),
        options,
        kube_namespace,
    )
}

// runs helm upgrade on the installed aptos-genesis release named "genesis"
// if a new "era" is specified, a new genesis will be created, and old resources will be destroyed
fn upgrade_genesis_helm(helm_repo: &str, options: &[String], kube_namespace: String) -> Result<()> {
    upgrade_helm_release(
        GENESIS_HELM_RELEASE_NAME.to_string(),
        format!("{}/aptos-genesis", helm_repo),
        options,
        kube_namespace,
    )
}

pub async fn uninstall_testnet_resources(kube_namespace: String) -> Result<()> {
    // delete kubernetes resources
    delete_k8s_cluster(kube_namespace).await?;
    println!("aptos-node resources for Forge removed");

    Ok(())
}

pub async fn reinstall_testnet_resources(
    kube_namespace: String,
    helm_repo: String,
    base_num_validators: usize,
    base_validator_image_tag: String,
    base_genesis_image_tag: String,
    require_validator_healthcheck: bool,
    genesis_modules_path: Option<String>,
) -> Result<String> {
    assert!(base_num_validators <= MAX_NUM_VALIDATORS);

    let new_era = get_new_era().unwrap();
    let kube_client = create_k8s_client().await;

    // get old values and cache it
    let tmp_dir = TempDir::new().expect("Could not create temp dir");
    let aptos_node_values_file = dump_helm_values_to_file(APTOS_NODE_HELM_RELEASE_NAME, &tmp_dir)?;
    let genesis_values_file = dump_helm_values_to_file(GENESIS_HELM_RELEASE_NAME, &tmp_dir)?;

    // just helm upgrade

    let aptos_node_upgrade_options = vec![
        // use the old values
        "-f".to_string(),
        aptos_node_values_file,
        "--set".to_string(),
        format!("chain.era={}", &new_era),
        "--set".to_string(),
        format!("numValidators={}", base_num_validators),
        "--set".to_string(),
        format!("imageTag={}", &base_genesis_image_tag),
    ];

    // TODO(rustielin): get the helm releases to be consistent
    upgrade_aptos_node_helm(
        APTOS_NODE_HELM_RELEASE_NAME.to_string(),
        &helm_repo,
        aptos_node_upgrade_options.as_slice(),
        kube_namespace.clone(),
    )?;

    let mut genesis_upgrade_options = vec![
        // use the old values
        "-f".to_string(),
        genesis_values_file,
        "--set".to_string(),
        format!("chain.era={}", &new_era),
        "--set".to_string(),
        format!("genesis.numValidators={}", base_num_validators),
        "--set".to_string(),
        // NOTE: remember to prepend 0x to the key
        format!("chain.rootKey=0x{}", DEFAULT_ROOT_KEY),
        "--set".to_string(),
        format!("imageTag={}", &base_genesis_image_tag),
    ];

    // run genesis from the directory in aptos/init image
    if let Some(genesis_modules_path) = genesis_modules_path {
        genesis_upgrade_options.extend([
            "--set".to_string(),
            format!("genesis.moveModulesDir={}", genesis_modules_path),
        ]);
    }

    // upgrade testnet
    upgrade_genesis_helm(
        &helm_repo,
        genesis_upgrade_options.as_slice(),
        kube_namespace.clone(),
    )?;

    // wait for genesis to run again, and get the updated validators
    wait_genesis_job(&kube_client, &new_era, &kube_namespace).await?;

    // healthcheck on each of the validators wait until they all healthy
    let unhealthy_nodes = if require_validator_healthcheck {
        let vals = get_validators(kube_client.clone(), &base_validator_image_tag)
            .await
            .unwrap();
        let all_nodes = vals.values().collect();
        nodes_healthcheck(all_nodes).await.unwrap()
    } else {
        vec![]
    };
    if !unhealthy_nodes.is_empty() {
        bail!("Unhealthy validators after cleanup: {:?}", unhealthy_nodes);
    }

    Ok(new_era)
}

fn get_new_era() -> Result<String> {
    // get a random new era to wipe the chain
    let mut rng = rand::thread_rng();
    let new_era: &str = &format!("fg{}", rng.gen::<u32>());
    println!("new chain era: {}", new_era);
    Ok(new_era.to_string())
}

pub async fn create_k8s_client() -> K8sClient {
    // get the client from the local kube context
    // TODO(rustielin|geekflyer): use proxy or port-forward to make REST API available
    let config_infer = Config::infer().await.unwrap();
    K8sClient::try_from(config_infer).unwrap()
}

pub fn scale_sts_replica(sts_name: &str, replica_num: u64) -> Result<()> {
    let scale_sts_args = [
        "scale",
        "sts",
        sts_name,
        &format!("--replicas={}", replica_num),
    ];
    println!("{:?}", scale_sts_args);
    let scale_output = Command::new(KUBECTL_BIN)
        .stdout(Stdio::inherit())
        .args(&scale_sts_args)
        .output()
        .expect("failed to scale sts replicas");
    assert!(
        scale_output.status.success(),
        "{}",
        String::from_utf8(scale_output.stderr).unwrap()
    );

    Ok(())
}

// XXX: quick helpers around helm operation on the default namespace
fn get_helm_status(helm_release_name: &str) -> Result<Value> {
    let status_args = [
        "status",
        helm_release_name,
        "--namespace",
        "default",
        "-o",
        "json",
    ];
    println!("{:?}", status_args);
    let raw_helm_values = Command::new(HELM_BIN)
        .args(&status_args)
        .output()
        .unwrap_or_else(|_| panic!("failed to helm status {}", helm_release_name));

    let helm_values = String::from_utf8(raw_helm_values.stdout).unwrap();
    serde_json::from_str(&helm_values)
        .map_err(|e| format_err!("failed to deserialize helm values: {}", e))
}

fn dump_helm_values_to_file(helm_release_name: &str, tmp_dir: &TempDir) -> Result<String> {
    // get aptos-node values
    let v: Value = get_helm_status(helm_release_name).unwrap();
    let config = &v["config"];

    // store the helm values for later use
    let file_path = tmp_dir
        .path()
        .join(format!("{}_status.json", helm_release_name));
    println!("Wrote helm values to: {:?}", &file_path);
    let mut file = File::create(file_path).expect("Could not create file in temp dir");
    file.write_all(&config.to_string().into_bytes())
        .expect("Could not write to file");
    let file_path_str = tmp_dir
        .path()
        .join(format!("{}_status.json", helm_release_name))
        .display()
        .to_string();
    Ok(file_path_str)
}
