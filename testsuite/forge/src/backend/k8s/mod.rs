// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    metrics::{record_cluster_spinup_phase, ClusterPhase},
    Factory, GenesisConfig, GenesisConfigFn, NodeConfigFn, Result, Swarm, Version,
    INDEXER_GRPC_DOCKER_IMAGE_REPO, VALIDATOR_DOCKER_IMAGE_REPO,
};
use anyhow::bail;
use futures::{future, FutureExt};
use log::info;
use rand::rngs::StdRng;
use serde_json::json;
use std::{
    convert::TryInto,
    num::NonZeroUsize,
    time::{Duration, Instant},
};

pub mod chaos;
pub mod chaos_schema;
mod cluster_helper;
pub mod constants;
mod fullnode;
pub mod kube_api;
pub mod node;
pub mod prometheus;
mod stateful_set;
mod swarm;

use super::{
    ForgeDeployerManager, FORGE_GENESIS_SHARED_BUCKET, FORGE_INDEXER_DEPLOYER_DOCKER_IMAGE_REPO,
    FORGE_PFN_DEPLOYER_DOCKER_IMAGE_REPO,
};
use aptos_sdk::crypto::ed25519::ED25519_PRIVATE_KEY_LENGTH;
pub use cluster_helper::*;
pub use constants::*;
pub use fullnode::*;
#[cfg(test)]
pub use kube_api::mocks::*;
pub use kube_api::*;
pub use node::K8sNode;
pub use stateful_set::*;
pub use swarm::*;

pub struct K8sFactory {
    root_key: [u8; ED25519_PRIVATE_KEY_LENGTH],
    image_tag: String,
    upgrade_image_tag: String,
    kube_namespace: String,
    use_port_forward: bool,
    reuse: bool,
    keep: bool,
    enable_haproxy: bool,
    enable_indexer: bool,
    enable_pfn: bool,
    deployer_profile: String,
}

impl K8sFactory {
    pub fn new(
        kube_namespace: String,
        image_tag: String,
        upgrade_image_tag: String,
        use_port_forward: bool,
        reuse: bool,
        keep: bool,
        enable_haproxy: bool,
        enable_indexer: bool,
        enable_pfn: bool,
        deployer_profile: String,
    ) -> Result<K8sFactory> {
        let root_key: [u8; ED25519_PRIVATE_KEY_LENGTH] =
            hex::decode(DEFAULT_ROOT_PRIV_KEY)?.try_into().unwrap();

        match kube_namespace.as_str() {
            "default" => {
                info!("Using the default kubernetes namespace");
            },
            s if s.starts_with("forge") => {
                info!("Using forge namespace: {}", s);
            },
            _ => {
                bail!(
                    "Invalid kubernetes namespace provided: {}. Use forge-*",
                    kube_namespace
                );
            },
        }

        Ok(Self {
            root_key,
            image_tag,
            upgrade_image_tag,
            kube_namespace,
            use_port_forward,
            reuse,
            keep,
            enable_haproxy,
            enable_indexer,
            enable_pfn,
            deployer_profile,
        })
    }
}

#[async_trait::async_trait]
impl Factory for K8sFactory {
    fn versions<'a>(&'a self) -> Box<dyn Iterator<Item = Version> + 'a> {
        let version = vec![
            Version::new(0, self.image_tag.clone()),
            Version::new(1, self.upgrade_image_tag.clone()),
        ];
        Box::new(version.into_iter())
    }

    async fn launch_swarm(
        &self,
        _rng: &mut StdRng,
        num_validators: NonZeroUsize,
        num_fullnodes: usize,
        init_version: &Version,
        genesis_version: &Version,
        genesis_config: Option<&GenesisConfig>,
        cleanup_duration: Duration,
        genesis_config_fn: Option<GenesisConfigFn>,
        node_config_fn: Option<NodeConfigFn>,
        existing_db_tag: Option<String>,
    ) -> Result<Box<dyn Swarm>> {
        let total_start = Instant::now();
        let namespace = &self.kube_namespace;

        let genesis_modules_path = match genesis_config {
            Some(config) => match config {
                GenesisConfig::Bundle(_) => {
                    bail!("k8s forge backend does not support raw bytes as genesis modules. please specify a path instead")
                },
                GenesisConfig::Path(path) => Some(path.clone()),
            },
            None => None,
        };

        let kube_client = create_k8s_client().await?;
        let (new_era, validators, fullnodes) = if self.reuse {
            let (validators, fullnodes) = match collect_running_nodes(
                &kube_client,
                self.kube_namespace.clone(),
                self.use_port_forward,
                self.enable_haproxy,
            )
            .await
            {
                Ok(res) => res,
                Err(e) => {
                    bail!(e);
                },
            };
            let new_era = None; // TODO: get the actual era
            (new_era, validators, fullnodes)
        } else {
            // Cleanup phase: clear the cluster of resources
            let cleanup_start = Instant::now();
            let cleanup_result =
                delete_k8s_resources(kube_client.clone(), &self.kube_namespace).await;
            record_cluster_spinup_phase(
                namespace,
                ClusterPhase::Cleanup,
                cleanup_start,
                cleanup_result.is_ok(),
            );
            cleanup_result?;

            // create the forge-management configmap before installing anything
            create_management_configmap(self.kube_namespace.clone(), self.keep, cleanup_duration)
                .await?;
            if let Some(existing_db_tag) = existing_db_tag {
                // TODO(prod-eng): For now we are managing PVs out of forge, and bind them manually
                // with the volume. Going forward we should consider automate this process.

                // The previously claimed PVs are in Released stage once the corresponding PVC is
                // gone. We reset its status to Available so they can be reused later.
                reset_persistent_volumes(&kube_client).await?;

                // We return early here if there are not enough PVs to claim.
                check_persistent_volumes(
                    kube_client.clone(),
                    num_validators.get() + num_fullnodes,
                    existing_db_tag,
                )
                .await?;
            }
            // try installing testnet resources, but clean up if it fails
            let new_era = generate_new_era();
            info!(
                "Creating new era {} in namespace {}",
                &new_era, &self.kube_namespace
            );

            // Testnet install phase
            let testnet_install_start = Instant::now();
            let testnet_namespace = self.kube_namespace.clone();
            let deploy_testnet_fut = async {
                let result = install_testnet_resources(
                    new_era.clone(),
                    self.kube_namespace.clone(),
                    num_validators.get(),
                    num_fullnodes,
                    format!("{}", init_version),
                    format!("{}", genesis_version),
                    genesis_modules_path,
                    self.use_port_forward,
                    self.enable_haproxy,
                    self.enable_indexer,
                    self.deployer_profile.clone(),
                    genesis_config_fn,
                    node_config_fn,
                    false,
                )
                .await;
                record_cluster_spinup_phase(
                    &testnet_namespace,
                    ClusterPhase::TestnetInstall,
                    testnet_install_start,
                    result.is_ok(),
                );
                result
            }
            .boxed();

            // Indexer deploy phase (if enabled)
            let indexer_deploy_start = Instant::now();
            let indexer_namespace = self.kube_namespace.clone();
            let enable_indexer = self.enable_indexer;
            let indexer_era = new_era.clone();
            let indexer_profile = self.deployer_profile.clone();
            let indexer_kube_namespace = self.kube_namespace.clone();
            let indexer_init_version = format!("{}", init_version);
            let indexer_kube_client = kube_client.clone();
            let deploy_indexer_fut = async move {
                if enable_indexer {
                    // NOTE: by default, use a deploy profile and no additional configuration values
                    let config = serde_json::from_value(json!({
                        "profile": indexer_profile,
                        "era": indexer_era,
                        "namespace": indexer_kube_namespace,
                        "indexer-grpc-values": {
                            "indexerGrpcImage": format!("{}:{}", INDEXER_GRPC_DOCKER_IMAGE_REPO, indexer_init_version),
                            "fullnodeConfig": {
                                "image": format!("{}:{}", VALIDATOR_DOCKER_IMAGE_REPO, indexer_init_version),
                            }
                        },
                    }))?;

                    let indexer_deployer = ForgeDeployerManager::new(
                        indexer_kube_client,
                        indexer_kube_namespace.clone(),
                        FORGE_INDEXER_DEPLOYER_DOCKER_IMAGE_REPO.to_string(),
                        None,
                    );
                    indexer_deployer.start(config).await?;
                    let result = indexer_deployer.wait_completed().await;
                    record_cluster_spinup_phase(
                        &indexer_namespace,
                        ClusterPhase::IndexerDeploy,
                        indexer_deploy_start,
                        result.is_ok(),
                    );
                    result
                } else {
                    Ok(())
                }
            }
            .boxed();

            // PFN deploy phase (if enabled)
            let pfn_deploy_start = Instant::now();
            let pfn_namespace = self.kube_namespace.clone();
            let enable_pfn = self.enable_pfn;
            let pfn_era = new_era.clone();
            let pfn_profile = self.deployer_profile.clone();
            let pfn_kube_namespace = self.kube_namespace.clone();
            let pfn_init_version = format!("{}", init_version);
            let pfn_kube_client = kube_client.clone();
            let deploy_pfn_fut = async move {
                if enable_pfn {
                    let genesis_bucket_path = format!(
                        "{}/{}/{}",
                        FORGE_GENESIS_SHARED_BUCKET, pfn_kube_namespace, pfn_era
                    );
                    let config = serde_json::from_value(json!({
                        "profile": pfn_profile,
                        "era": pfn_era,
                        "namespace": pfn_kube_namespace,
                        "pfn-values": {
                            "image": format!("{}:{}", VALIDATOR_DOCKER_IMAGE_REPO, pfn_init_version),
                            "genesis_bucket_path": genesis_bucket_path,
                        },
                    }))?;

                    let pfn_deployer = ForgeDeployerManager::new(
                        pfn_kube_client,
                        pfn_kube_namespace.clone(),
                        FORGE_PFN_DEPLOYER_DOCKER_IMAGE_REPO.to_string(),
                        None,
                    );
                    pfn_deployer.start(config).await?;
                    let result = pfn_deployer.wait_completed().await;
                    record_cluster_spinup_phase(
                        &pfn_namespace,
                        ClusterPhase::PfnDeploy,
                        pfn_deploy_start,
                        result.is_ok(),
                    );
                    result
                } else {
                    Ok(())
                }
            }
            .boxed();

            // Join on testnet, indexer, and PFN deployment futures in parallel.
            // try_join3 ensures fail-fast: if any deployer fails, the others are cancelled.
            let (validators, fullnodes) =
                match future::try_join3(deploy_testnet_fut, deploy_indexer_fut, deploy_pfn_fut)
                    .await
                {
                    Ok((deploy_testnet_ret, _, _)) => deploy_testnet_ret,
                    Err(e) => {
                        uninstall_testnet_resources(self.kube_namespace.clone()).await?;
                        bail!(e);
                    },
                };

            (Some(new_era), validators, fullnodes)
        };

        // Health check phase: K8sSwarm::new includes health checks
        let health_check_start = Instant::now();
        let swarm_result = K8sSwarm::new(
            &self.root_key,
            &self.image_tag,
            &self.upgrade_image_tag,
            &self.kube_namespace,
            validators,
            fullnodes,
            self.keep,
            new_era,
            self.use_port_forward,
            self.enable_indexer,
        )
        .await;
        record_cluster_spinup_phase(
            namespace,
            ClusterPhase::HealthCheck,
            health_check_start,
            swarm_result.is_ok(),
        );

        let swarm = swarm_result?;

        // Record total spin-up time
        record_cluster_spinup_phase(namespace, ClusterPhase::Total, total_start, true);

        Ok(Box::new(swarm))
    }
}
