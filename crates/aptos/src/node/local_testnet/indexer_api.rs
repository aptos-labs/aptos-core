// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    health_checker::HealthChecker,
    traits::{PostHealthyStep, ServiceManager, ShutdownStep},
    utils::{confirm_docker_available, delete_container, pull_docker_image},
    RunLocalTestnet,
};
use crate::node::local_testnet::utils::{
    get_docker, setup_docker_logging, KillContainerShutdownStep,
};
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use bollard::{
    container::{Config, CreateContainerOptions, StartContainerOptions, WaitContainerOptions},
    models::{HostConfig, PortBinding},
};
use clap::Parser;
use futures::TryStreamExt;
use maplit::{hashmap, hashset};
use reqwest::Url;
use std::{collections::HashSet, path::PathBuf};
use tracing::{info, warn};

const INDEXER_API_CONTAINER_NAME: &str = "indexer-api";
const HASURA_IMAGE: &str = "hasura/graphql-engine:v2.33.0";

/// This Hasura metadata origintes from the aptos-indexer-processors repo.
///
/// This metadata is from revision: 1b8e14d9669258f797403e2b38da9ea5aea29e35.
///
/// The metadata file is not taken verbatim, it is currently edited by hand to remove
/// any references to tables that aren't created by the Rust processor migrations.
/// This works fine today since all the key processors you'd need in a local testnet
/// are in the set of processors written in Rust. If this changes, we can explore
/// alternatives, e.g. running processors in other languages using containers.
const HASURA_METADATA: &str = include_str!("hasura_metadata.json");

/// Args related to running an indexer API for the local testnet.
#[derive(Debug, Parser)]
pub struct IndexerApiArgs {
    /// If set, we will run a postgres DB using Docker (unless
    /// --use-host-postgres is set), run the standard set of indexer processors (see
    /// --processors) and configure them to write to this DB, and run an API that lets
    /// you access the data they write to storage. This is opt in because it requires
    /// Docker to be installed in the host system.
    #[clap(long, conflicts_with = "no_txn_stream")]
    pub with_indexer_api: bool,

    /// The port at which to run the indexer API.
    #[clap(long, default_value_t = 8090)]
    pub indexer_api_port: u16,
}

#[derive(Clone, Debug)]
pub struct IndexerApiManager {
    indexer_api_port: u16,
    prerequisite_health_checkers: HashSet<HealthChecker>,
    test_dir: PathBuf,
    postgres_connection_string: String,
}

impl IndexerApiManager {
    pub fn new(
        args: &RunLocalTestnet,
        prerequisite_health_checkers: HashSet<HealthChecker>,
        test_dir: PathBuf,
        postgres_connection_string: String,
    ) -> Result<Self> {
        Ok(Self {
            indexer_api_port: args.indexer_api_args.indexer_api_port,
            prerequisite_health_checkers,
            test_dir,
            postgres_connection_string,
        })
    }
}

#[async_trait]
impl ServiceManager for IndexerApiManager {
    fn get_name(&self) -> String {
        "Indexer API".to_string()
    }

    async fn pre_run(&self) -> Result<()> {
        // Confirm Docker is available.
        confirm_docker_available().await?;

        // Delete any existing indexer API container we find.
        delete_container(INDEXER_API_CONTAINER_NAME).await?;

        // Pull the image here so it is not subject to the 30 second startup timeout.
        pull_docker_image(HASURA_IMAGE).await?;

        // Warn the user about DOCKER_DEFAULT_PLATFORM.
        if let Ok(var) = std::env::var("DOCKER_DEFAULT_PLATFORM") {
            eprintln!(
                "WARNING: DOCKER_DEFAULT_PLATFORM is set to {}. This may cause problems \
                with running the indexer API. If it fails to start up, try unsetting \
                this env var.\n",
                var
            );
        }

        Ok(())
    }

    fn get_healthchecks(&self) -> HashSet<HealthChecker> {
        hashset! {HealthChecker::Http(
            Url::parse(&format!("http://127.0.0.1:{}", self.indexer_api_port)).unwrap(),
            self.get_name(),
        )}
    }

    fn get_prerequisite_health_checkers(&self) -> HashSet<&HealthChecker> {
        self.prerequisite_health_checkers.iter().collect()
    }

    async fn run_service(self: Box<Self>) -> Result<()> {
        setup_docker_logging(&self.test_dir, "indexer-api", INDEXER_API_CONTAINER_NAME)?;

        // Unconditionally use host.docker.internal instead of 127.0.0.1 to access the
        // host system. This currently works out of the box on Docker for Desktop on
        // Mac and Windows. On Linux, this requires that you bind the name to the host
        // gateway, which we do below.
        let postgres_connection_string = self
            .postgres_connection_string
            .replace("127.0.0.1", "host.docker.internal");

        info!(
            "Using postgres connection string: {}",
            postgres_connection_string
        );

        let options = Some(CreateContainerOptions {
            name: INDEXER_API_CONTAINER_NAME,
            ..Default::default()
        });

        let exposed_ports = Some(hashmap! {self.indexer_api_port.to_string() => hashmap!{}});
        let mut host_config = HostConfig {
            port_bindings: Some(hashmap! {
                self.indexer_api_port.to_string() => Some(vec![PortBinding {
                    host_ip: Some("127.0.0.1".to_string()),
                    host_port: Some(self.indexer_api_port.to_string()),
                }]),
            }),
            ..Default::default()
        };

        if cfg!(target_os = "linux") {
            host_config.extra_hosts = Some(vec!["host.docker.internal:host-gateway".to_string()]);
        }

        let config = Config {
            image: Some(HASURA_IMAGE.to_string()),
            tty: Some(true),
            exposed_ports,
            host_config: Some(host_config),
            env: Some(vec![
                format!("PG_DATABASE_URL={}", postgres_connection_string),
                format!(
                    "HASURA_GRAPHQL_METADATA_DATABASE_URL={}",
                    postgres_connection_string
                ),
                format!("INDEXER_V2_POSTGRES_URL={}", postgres_connection_string),
                "HASURA_GRAPHQL_DEV_MODE=true".to_string(),
                "HASURA_GRAPHQL_ENABLE_CONSOLE=true".to_string(),
                "HASURA_GRAPHQL_CONSOLE_ASSETS_DIR=/srv/console-assets".to_string(),
                format!("HASURA_GRAPHQL_SERVER_PORT={}", self.indexer_api_port),
            ]),
            ..Default::default()
        };

        info!("Starting indexer API with this config: {:#?}", config);

        let docker = get_docker()?;

        let id = docker.create_container(options, config).await?.id;

        info!("Created container with this ID: {}", id);

        docker
            .start_container(&id, None::<StartContainerOptions<&str>>)
            .await
            .context("Failed to start indexer API container")?;

        info!("Started container {}", id);

        // Wait for the container to stop, which it never should unless we receive
        // ctrl-c.
        let wait = docker
            .wait_container(
                &id,
                Some(WaitContainerOptions {
                    condition: "not-running",
                }),
            )
            .try_collect::<Vec<_>>()
            .await
            .context("Failed to wait on indexer API container")?;

        warn!("Indexer API stopped: {:?}", wait.last());

        Ok(())
    }

    fn get_post_healthy_steps(&self) -> Vec<Box<dyn PostHealthyStep>> {
        /// There is no good way to apply Hasura metadata (the JSON format, anyway) to
        /// an instance of Hasura in a container at startup:
        ///
        /// https://github.com/hasura/graphql-engine/issues/8423
        ///
        /// As such, the only way to do it is to apply it via the API after startup.
        /// That is what this post healthy step does.
        #[derive(Debug)]
        struct PostMetdataPostHealthyStep {
            pub indexer_api_port: u16,
        }

        #[async_trait]
        impl PostHealthyStep for PostMetdataPostHealthyStep {
            async fn run(self: Box<Self>) -> Result<()> {
                post_metadata(HASURA_METADATA, self.indexer_api_port)
                    .await
                    .context("Failed to apply Hasura metadata for Indexer API")?;
                Ok(())
            }
        }

        vec![Box::new(PostMetdataPostHealthyStep {
            indexer_api_port: self.indexer_api_port,
        })]
    }

    fn get_shutdown_steps(&self) -> Vec<Box<dyn ShutdownStep>> {
        // Unfortunately the Hasura container does not shut down when the CLI does and
        // there doesn't seem to be a good way to make it do so. To work around this,
        // we register a step that will delete the container on shutdown.
        // Read more here: https://stackoverflow.com/q/77171786/3846032.
        vec![Box::new(KillContainerShutdownStep::new(
            INDEXER_API_CONTAINER_NAME,
        ))]
    }
}

/// This submits a POST request to apply metadata to a Hasura API.
async fn post_metadata(metadata_content: &str, port: u16) -> Result<()> {
    let url = format!("http://127.0.0.1:{}/v1/metadata", port);
    let client = reqwest::Client::new();

    // Parse the metadata content as JSON.
    let metadata_json: serde_json::Value = serde_json::from_str(metadata_content)?;

    // Construct the payload.
    let mut payload = serde_json::Map::new();
    payload.insert(
        "type".to_string(),
        serde_json::Value::String("replace_metadata".to_string()),
    );
    payload.insert("args".to_string(), metadata_json);

    // Send the POST request.
    let response = client.post(url).json(&payload).send().await?;

    // Check that `is_consistent` is true in the response.
    let json = response.json().await?;
    check_is_consistent(&json)?;

    Ok(())
}

/// This checks the response from the API to confirm the metadata was applied
/// successfully.
fn check_is_consistent(json: &serde_json::Value) -> Result<()> {
    if let Some(obj) = json.as_object() {
        if let Some(is_consistent_val) = obj.get("is_consistent") {
            if is_consistent_val.as_bool() == Some(true) {
                return Ok(());
            }
        }
    }

    Err(anyhow!(
        "Something went wrong applying the Hasura metadata, perhaps it is not consistent with the DB. Response: {:#?}",
        json
    ))
}
