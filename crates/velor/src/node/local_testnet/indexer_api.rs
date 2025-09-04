// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    docker::{
        delete_container, get_docker, pull_docker_image, setup_docker_logging,
        StopContainerShutdownStep, CONTAINER_NETWORK_NAME,
    },
    health_checker::HealthChecker,
    traits::{PostHealthyStep, ServiceManager, ShutdownStep},
    RunLocalnet,
};
use anyhow::{anyhow, Context, Result};
pub use velor_localnet::indexer_api::{
    make_hasura_metadata_request, post_metadata, HASURA_IMAGE, HASURA_METADATA,
};
use async_trait::async_trait;
use bollard::{
    container::{Config, CreateContainerOptions, StartContainerOptions, WaitContainerOptions},
    models::{HostConfig, PortBinding},
};
use clap::Parser;
use futures::TryStreamExt;
use maplit::{hashmap, hashset};
use reqwest::Url;
use std::{collections::HashSet, path::PathBuf, time::Duration};
use tracing::{info, warn};

const INDEXER_API_CONTAINER_NAME: &str = "local-testnet-indexer-api";

/// Args related to running an indexer API for the localnet.
#[derive(Debug, Parser)]
pub struct IndexerApiArgs {
    /// If set, we will run a postgres DB using Docker (unless --use-host-postgres is
    /// set), run the standard set of indexer processors (see --processors), and
    /// configure them to write to this DB, and run an API that lets you access the data
    /// they write to storage. This is opt in because it requires Docker to be installed
    /// on the host system.
    #[clap(long, conflicts_with = "no_txn_stream")]
    pub with_indexer_api: bool,

    /// The port at which to run the indexer API.
    #[clap(long, default_value_t = 8090)]
    pub indexer_api_port: u16,

    /// If set we will assume a Hasura instance is running at the given URL rather than
    /// running our own.
    ///
    /// If set, we will not run the indexer API, and will instead assume that a Hasura
    /// instance is running at the given URL. We will wait for it to become healthy by
    /// waiting for / to return 200 and then apply the Hasura metadata. The URL should
    /// look something like this: http://127.0.0.1:8090, assuming the Hasura instance is
    /// running at port 8090. When the localnet shuts down, we will not attempt to stop
    /// the Hasura instance, this is up to you to handle. If you're using this, you
    /// should probably use `--use-host-postgres` as well, otherwise you won't be able
    /// to start your Hasura instance because the DB we create won't exist yet.
    #[clap(long)]
    pub existing_hasura_url: Option<Url>,

    /// If set, we will not try to apply the Hasura metadata.
    #[clap(long)]
    pub skip_metadata_apply: bool,
}

#[derive(Clone, Debug)]
pub struct IndexerApiManager {
    indexer_api_port: u16,
    existing_hasura_url: Option<Url>,
    skip_metadata_apply: bool,
    prerequisite_health_checkers: HashSet<HealthChecker>,
    test_dir: PathBuf,
    postgres_connection_string: String,
}

impl IndexerApiManager {
    pub fn new(
        args: &RunLocalnet,
        prerequisite_health_checkers: HashSet<HealthChecker>,
        test_dir: PathBuf,
        postgres_connection_string: String,
    ) -> Result<Self> {
        Ok(Self {
            indexer_api_port: args.indexer_api_args.indexer_api_port,
            existing_hasura_url: args.indexer_api_args.existing_hasura_url.clone(),
            skip_metadata_apply: args.indexer_api_args.skip_metadata_apply,
            prerequisite_health_checkers,
            test_dir,
            postgres_connection_string,
        })
    }

    pub fn get_url(&self) -> Url {
        match &self.existing_hasura_url {
            Some(url) => url.clone(),
            None => Url::parse(&format!("http://127.0.0.1:{}", self.indexer_api_port)).unwrap(),
        }
    }
}

#[async_trait]
impl ServiceManager for IndexerApiManager {
    fn get_name(&self) -> String {
        "Indexer API".to_string()
    }

    async fn pre_run(&self) -> Result<()> {
        if self.existing_hasura_url.is_some() {
            return Ok(());
        }

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
        let mut checkers = hashset! {
            // This first one just checks if the API is up at all.
            HealthChecker::Http(self.get_url(), "Indexer API".to_string()),
        };
        if !self.skip_metadata_apply {
            // This second one checks if the metadata is applied.
            checkers.insert(HealthChecker::IndexerApiMetadata(self.get_url()));
        }
        checkers
    }

    fn get_prerequisite_health_checkers(&self) -> HashSet<&HealthChecker> {
        self.prerequisite_health_checkers.iter().collect()
    }

    async fn run_service(self: Box<Self>) -> Result<()> {
        // If we're using an existing Hasura instance we just do nothing. If the Hasura
        // instance becomes unhealthy we print an error and exit.
        if let Some(url) = self.existing_hasura_url {
            info!("Using existing Hasura instance at {}", url);
            // Periodically check that the Hasura instance is healthy.
            let checker = HealthChecker::Http(url.clone(), "Indexer API".to_string());
            loop {
                if let Err(e) = checker.wait(None).await {
                    eprintln!(
                        "Existing Hasura instance at {} became unhealthy: {}",
                        url, e
                    );
                    break;
                }
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            return Ok(());
        }

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
        if self.skip_metadata_apply {
            return vec![];
        }

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
        if self.existing_hasura_url.is_some() {
            return vec![];
        }

        // Unfortunately the Hasura container does not shut down when the CLI does and
        // there doesn't seem to be a good way to make it do so. To work around this,
        // we register a step that will stop the container on shutdown.
        // Read more here: https://stackoverflow.com/q/77171786/3846032.
        vec![Box::new(StopContainerShutdownStep::new(
            INDEXER_API_CONTAINER_NAME,
        ))]
    }
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
