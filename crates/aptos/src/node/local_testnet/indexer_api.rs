// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    docker::{
        delete_container, get_docker, pull_docker_image, setup_docker_logging,
        StopContainerShutdownStep, CONTAINER_NETWORK_NAME,
    },
    health_checker::HealthChecker,
    traits::{PostHealthyStep, ServiceManager, ShutdownStep},
    RunLocalTestnet,
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

const INDEXER_API_CONTAINER_NAME: &str = "local-testnet-indexer-api";
const HASURA_IMAGE: &str = "hasura/graphql-engine:v2.36.1";

/// This Hasura metadata originates from the aptos-indexer-processors repo.
///
/// This metadata is from revision: 4801acae7aea30d7e96bbfbe5ec5b04056dfa4cf.
///
/// The metadata file is not taken verbatim, it is currently edited by hand to remove
/// any references to tables that aren't created by the Rust processor migrations.
///
/// To arrive at the final edited file I normally start with the new metadata file,
/// try to start the local testnet, and check .aptos/testnet/main/tracing.log to
/// see what error Hasura returned. Remove the culprit from the metadata, which is
/// generally a few tables and relations to those tables, and try again. Repeat until
/// it accepts the metadata.
///
/// This works fine today since all the key processors you'd need in a local testnet
/// are in the set of processors written in Rust. If this changes, we can explore
/// alternatives, e.g. running processors in other languages using containers.
const HASURA_METADATA: &str = include_str!("hasura_metadata.json");

/// Args related to running an indexer API for the local testnet.
#[derive(Debug, Parser)]
pub struct IndexerApiArgs {
    /// If set, we will run a postgres DB using Docker (unless
    /// --use-host-postgres is set), run the standard set of indexer processors (see
    /// --processors), and configure them to write to this DB, and run an API that lets
    /// you access the data they write to storage. This is opt in because it requires
    /// Docker to be installed on the host system.
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

    pub fn get_url(&self) -> Url {
        Url::parse(&format!("http://127.0.0.1:{}", self.indexer_api_port)).unwrap()
    }
}

#[async_trait]
impl ServiceManager for IndexerApiManager {
    fn get_name(&self) -> String {
        "Indexer API".to_string()
    }

    async fn pre_run(&self) -> Result<()> {
        // Confirm Docker is available.
        get_docker().await?;

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

    /// In this case we we return two HealthCheckers, one for whether the Hasura API
    /// is up at all and one for whether the metadata is applied.
    fn get_health_checkers(&self) -> HashSet<HealthChecker> {
        hashset! {
            // This first one just checks if the API is up at all.
            HealthChecker::Http(self.get_url(), "Indexer API".to_string()),
            // This second one checks if the metadata is applied.
            HealthChecker::IndexerApiMetadata(self.get_url()),
        }
    }

    fn get_prerequisite_health_checkers(&self) -> HashSet<&HealthChecker> {
        self.prerequisite_health_checkers.iter().collect()
    }

    async fn run_service(self: Box<Self>) -> Result<()> {
        setup_docker_logging(&self.test_dir, "indexer-api", INDEXER_API_CONTAINER_NAME)?;

        // This is somewhat hard to maintain. If it requires any further maintenance we
        // should just delete support for using Postgres on the host system.
        let (postgres_connection_string, network_mode) =
            // When connecting to postgres on the host via an IP from inside a
            // container, we need to instead connect to host.docker.internal.
            // There is no need to bind to a Docker network in this case.
            if self.postgres_connection_string.contains("127.0.0.1") {
                (
                    self.postgres_connection_string
                        .replace("127.0.0.1", "host.docker.internal"),
                    None,
                )
            } else {
                // Otherwise we use the standard connection string (containing the name
                // of the container) and bind to the Docker network we created earlier
                // in the Postgres pre_run steps.
                (
                    self.postgres_connection_string,
                    Some(CONTAINER_NETWORK_NAME.to_string()),
                )
            };

        let exposed_ports = Some(hashmap! {self.indexer_api_port.to_string() => hashmap!{}});
        let host_config = HostConfig {
            // Connect the container to the network we made in the postgres pre_run.
            // This allows the indexer API to access the postgres container without
            // routing through the host network.
            network_mode,
            // This is necessary so connecting to the host postgres works on Linux.
            extra_hosts: Some(vec!["host.docker.internal:host-gateway".to_string()]),
            port_bindings: Some(hashmap! {
                self.indexer_api_port.to_string() => Some(vec![PortBinding {
                    host_ip: Some("127.0.0.1".to_string()),
                    host_port: Some(self.indexer_api_port.to_string()),
                }]),
            }),
            ..Default::default()
        };

        let docker = get_docker().await?;

        info!(
            "Using postgres connection string: {}",
            postgres_connection_string
        );

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
                // See the docs for the image, this is a magic path inside the
                // container where they have already bundled in the UI assets.
                "HASURA_GRAPHQL_CONSOLE_ASSETS_DIR=/srv/console-assets".to_string(),
                format!("HASURA_GRAPHQL_SERVER_PORT={}", self.indexer_api_port),
            ]),
            ..Default::default()
        };

        let options = Some(CreateContainerOptions {
            name: INDEXER_API_CONTAINER_NAME,
            ..Default::default()
        });

        info!("Starting indexer API with this config: {:?}", config);

        let id = docker.create_container(options, config).await?.id;

        info!("Created container for indexer API with this ID: {}", id);

        docker
            .start_container(&id, None::<StartContainerOptions<&str>>)
            .await
            .context("Failed to start indexer API container")?;

        info!("Started indexer API container {}", id);

        // Wait for the container to stop (which it shouldn't).
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
            pub indexer_api_url: Url,
        }

        #[async_trait]
        impl PostHealthyStep for PostMetdataPostHealthyStep {
            async fn run(self: Box<Self>) -> Result<()> {
                post_metadata(self.indexer_api_url, HASURA_METADATA)
                    .await
                    .context("Failed to apply Hasura metadata for Indexer API")?;
                Ok(())
            }
        }

        vec![Box::new(PostMetdataPostHealthyStep {
            indexer_api_url: self.get_url(),
        })]
    }

    fn get_shutdown_steps(&self) -> Vec<Box<dyn ShutdownStep>> {
        // Unfortunately the Hasura container does not shut down when the CLI does and
        // there doesn't seem to be a good way to make it do so. To work around this,
        // we register a step that will stop the container on shutdown.
        // Read more here: https://stackoverflow.com/q/77171786/3846032.
        vec![Box::new(StopContainerShutdownStep::new(
            INDEXER_API_CONTAINER_NAME,
        ))]
    }
}

/// This submits a POST request to apply metadata to a Hasura API.
async fn post_metadata(url: Url, metadata_content: &str) -> Result<()> {
    // Parse the metadata content as JSON.
    let metadata_json: serde_json::Value = serde_json::from_str(metadata_content)?;

    // Make the request.
    info!("Submitting request to apply Hasura metadata");
    let response =
        make_hasura_metadata_request(url, "replace_metadata", Some(metadata_json)).await?;
    info!(
        "Received response for applying Hasura metadata: {:?}",
        response
    );

    // Confirm that the metadata was applied successfully and there is no inconsistency
    // between the schema and the underlying DB schema.
    if let Some(obj) = response.as_object() {
        if let Some(is_consistent_val) = obj.get("is_consistent") {
            if is_consistent_val.as_bool() == Some(true) {
                return Ok(());
            }
        }
    }

    Err(anyhow!(
        "Something went wrong applying the Hasura metadata, perhaps it is not consistent with the DB. Response: {:#?}",
        response
    ))
}

/// This confirms that the metadata has been applied. We use this in the health
/// checker.
pub async fn confirm_metadata_applied(url: Url) -> Result<()> {
    // Make the request.
    info!("Confirming Hasura metadata applied...");
    let response = make_hasura_metadata_request(url, "export_metadata", None).await?;
    info!(
        "Received response for confirming Hasura metadata applied: {:?}",
        response
    );

    // If the sources field is set it means the metadata was applied successfully.
    if let Some(obj) = response.as_object() {
        if let Some(sources) = obj.get("sources") {
            if let Some(sources) = sources.as_array() {
                if !sources.is_empty() {
                    return Ok(());
                }
            }
        }
    }

    Err(anyhow!(
        "The Hasura metadata has not been applied yet. Response: {:#?}",
        response
    ))
}

/// The /v1/metadata endpoint supports a few different operations based on the `type`
/// field in the request body. All requests have a similar format, with these `type`
/// and `args` fields.
async fn make_hasura_metadata_request(
    mut url: Url,
    typ: &str,
    args: Option<serde_json::Value>,
) -> Result<serde_json::Value> {
    let client = reqwest::Client::new();

    // Update the query path.
    url.set_path("/v1/metadata");

    // Construct the payload.
    let mut payload = serde_json::Map::new();
    payload.insert(
        "type".to_string(),
        serde_json::Value::String(typ.to_string()),
    );

    // If args is provided, use that. Otherwise use an empty object. We have to set it
    // no matter what because the API expects the args key to be set.
    let args = match args {
        Some(args) => args,
        None => serde_json::Value::Object(serde_json::Map::new()),
    };
    payload.insert("args".to_string(), args);

    // Send the POST request.
    let response = client.post(url).json(&payload).send().await?;

    // Return the response as a JSON value.
    response
        .json()
        .await
        .context("Failed to parse response as JSON")
}
