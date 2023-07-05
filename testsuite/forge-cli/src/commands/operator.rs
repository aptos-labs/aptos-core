// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::{time::Duration, num::NonZeroUsize};

use anyhow::Result;
use aptos_forge::{
    cleanup_cluster_with_management, install_testnet_resources, set_stateful_set_image_tag,
    uninstall_testnet_resources, K8sFactory, NAMESPACE_CLEANUP_DURATION_BUFFER_SECS, Factory,
};
use clap::{Parser, Subcommand};
use rand::{SeedableRng, rngs::OsRng, Rng};

use crate::utils::generate_random_namespace;

#[derive(Subcommand, Debug)]
pub enum OperatorCommand {
    /// Set the image tag for a node in the cluster
    SetNodeImageTag(SetNodeImageTag),
    /// Clean up an existing cluster
    CleanUp(CleanUp),
    /// Resize an existing cluster
    Resize(Resize),
    /// Create a new cluster
    Create(Create),
}

impl OperatorCommand {
    pub async fn run(self) -> Result<()> {
        match self {
            OperatorCommand::SetNodeImageTag(set_stateful_set_image_tag_config) => {
                set_stateful_set_image_tag(
                    set_stateful_set_image_tag_config.stateful_set_name,
                    set_stateful_set_image_tag_config.container_name,
                    set_stateful_set_image_tag_config.image_tag,
                    set_stateful_set_image_tag_config.namespace,
                )
                .await?;
            },
            OperatorCommand::CleanUp(cleanup) => {
                if let Some(namespace) = cleanup.namespace {
                    uninstall_testnet_resources(namespace).await?;
                } else {
                    cleanup_cluster_with_management().await?;
                }
            },
            OperatorCommand::Resize(resize) => {
                install_testnet_resources(
                    resize.namespace,
                    resize.num_validators,
                    resize.num_fullnodes,
                    resize.validator_image_tag,
                    resize.testnet_image_tag,
                    resize.move_modules_dir,
                    !resize.connect_directly,
                    resize.enable_haproxy,
                    None,
                    None,
                )
                .await?;
            },
            OperatorCommand::Create(_) => {
                let namespace = generate_random_namespace()?;
                let image_tag = "aptos-node-v1.5.1".to_string();
                let upgrade_image_tag = "aptos-node-v1.5.1".to_string();
                let port_forward = true;
                let reuse = false;
                let keep = true;
                let enable_haproxy = false;

                let factory = K8sFactory::new(
                    namespace,
                    image_tag,
                    upgrade_image_tag,
                    // We want to port forward if we're running locally because local means we're not in cluster
                    port_forward,
                    reuse,
                    keep,
                    enable_haproxy,
                )?;

                let versions = factory.versions();

                let mut rng = ::rand::rngs::StdRng::from_seed(OsRng.gen());
                let initial_validator_count: NonZeroUsize = NonZeroUsize::new(5).unwrap();
                let initial_fullnode_count: usize = 5;
                let initial_version = versions.max().unwrap();
                let genesis_version = initial_version.clone();
                let genesis_config = None;
                let global_duration = Duration::from_secs(300 + NAMESPACE_CLEANUP_DURATION_BUFFER_SECS);
                let genesis_helm_config_fn = None;
                let node_helm_config_fn = None;
                let existing_db_tag = None;

                let mut swarm = factory.launch_swarm(
                    &mut rng,
                    initial_validator_count,
                    initial_fullnode_count,
                    &initial_version,
                    &genesis_version,
                    genesis_config.as_ref(),
                    global_duration,
                    genesis_helm_config_fn.clone(),
                    node_helm_config_fn.clone(),
                    existing_db_tag.clone(),
                ).await?;

                swarm.health_check().await?;
            }
        }
        Ok(())
    }
}

#[derive(Parser, Debug)]
pub struct SetNodeImageTag {
    #[clap(long, help = "The name of the node StatefulSet to update")]
    stateful_set_name: String,
    #[clap(long, help = "The name of the container to update")]
    container_name: String,
    #[clap(long, help = "The docker image tag to use for the node")]
    image_tag: String,
    #[clap(long, help = "The kubernetes namespace to clean up")]
    namespace: String,
}

#[derive(Parser, Debug)]
pub struct CleanUp {
    #[clap(
        long,
        help = "The kubernetes namespace to clean up. If unset, attemps to cleanup all by using forge-management configmaps"
    )]
    namespace: Option<String>,
}

#[derive(Parser, Debug)]
pub struct Resize {
    #[clap(long, help = "The kubernetes namespace to resize")]
    namespace: String,
    #[clap(long, default_value_t = 30)]
    num_validators: usize,
    #[clap(long, default_value_t = 1)]
    num_fullnodes: usize,
    #[clap(
        long,
        help = "Override the image tag used for validators",
        default_value = "devnet"
    )]
    validator_image_tag: String,
    #[clap(
        long,
        help = "Override the image tag used for testnet-specific components",
        default_value = "devnet"
    )]
    testnet_image_tag: String,
    #[clap(
        long,
        help = "Path to flattened directory containing compiled Move modules"
    )]
    move_modules_dir: Option<String>,
    #[clap(
        long,
        help = "If set, dont use kubectl port forward to access the cluster"
    )]
    connect_directly: bool,
    #[clap(long, help = "If set, enables HAProxy for each of the validators")]
    enable_haproxy: bool,
}

#[derive(Parser, Debug)]
pub struct Create {
}
