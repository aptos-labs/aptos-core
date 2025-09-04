// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    docker_common::create_start_and_inspect_container,
    postgres::get_postgres_connection_string_within_docker_network,
};
use crate::{
    common::{make_shared, ArcError, IP_LOCAL_HOST},
    no_panic_println,
};
use anyhow::{anyhow, Context, Result};
use velor_localnet::{
    health_checker::HealthChecker,
    indexer_api::{post_metadata, HASURA_IMAGE, HASURA_METADATA},
};
use bollard::{
    container::{CreateContainerOptions, WaitContainerOptions},
    secret::{ContainerInspectResponse, HostConfig, PortBinding},
    Docker,
};
use futures::TryStreamExt;
use maplit::hashmap;
use std::{future::Future, sync::Arc};
use tokio::{sync::Mutex, try_join};
use tokio_util::sync::CancellationToken;
use url::Url;
use uuid::Uuid;

const HASURA_DEFAULT_PORT: u16 = 8080;

/// Extracts the host port assigned to the hasura container from its inspection data.
fn get_hasura_assigned_port(container_info: &ContainerInspectResponse) -> Option<u16> {
    if let Some(port_bindings) = container_info
        .network_settings
        .as_ref()
        .and_then(|ns| ns.ports.as_ref())
    {
        if let Some(Some(bindings)) = port_bindings.get(&format!("{}/tcp", HASURA_DEFAULT_PORT)) {
            if let Some(binding) = bindings.first() {
                return binding
                    .host_port
                    .as_ref()
                    .and_then(|port| port.parse::<u16>().ok());
            }
        }
    }
    None
}

/// Returns the Docker container options and configuration to start a hasura container with
/// - The container bound to a Docker network (`network_name`).
/// - [`HASURA_DEFAULT_PORT`] mapped to a random OS-assigned host port.
fn create_container_options_and_config(
    instance_id: Uuid,
    network_name: String,
) -> (
    CreateContainerOptions<String>,
    bollard::container::Config<String>,
) {
    let postgres_connection_string =
        get_postgres_connection_string_within_docker_network(instance_id);

    let host_config = HostConfig {
        // Connect the container to the network we made in the postgres pre_run.
        // This allows the indexer API to access the postgres container without
        // routing through the host network.
        network_mode: Some(network_name),
        // This is necessary so connecting to the host postgres works on Linux.
        extra_hosts: Some(vec!["host.docker.internal:host-gateway".to_string()]),
        port_bindings: Some(hashmap! {
            HASURA_DEFAULT_PORT.to_string() => Some(vec![PortBinding {
                host_ip: Some(IP_LOCAL_HOST.to_string()),
                host_port: None,
            }]),
        }),
        ..Default::default()
    };

    let config = bollard::container::Config {
        image: Some(HASURA_IMAGE.to_string()),
        tty: Some(true),
        exposed_ports: Some(hashmap! {HASURA_DEFAULT_PORT.to_string() => hashmap!{}}),
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
            format!("HASURA_GRAPHQL_SERVER_PORT={}", HASURA_DEFAULT_PORT),
        ]),
        ..Default::default()
    };

    let options = CreateContainerOptions {
        name: format!("velor-workspace-{}-indexer-api", instance_id),
        ..Default::default()
    };

    (options, config)
}

/// Starts the indexer API service, running in a docker container.
///
/// Prerequisites
/// - Previous task to create the docker network
///   - Needs to be the same one the postgres container connects to
/// - Postgres DB (container)
/// - Indexer processors
///
/// The function returns three futures:
/// - One that resolves to the host port that can be used to access the indexer API service
///   when it's fully up.
/// - One that resolves when the container stops  (which it should not under normal operation).
/// - A cleanup task that stops the container and removes the associated data volume.
///
/// As the caller, you should always await the cleanup task when you are ready to shutdown the
/// service. The cleanup is a "best-effort" operation -- success is not guaranteed
/// as it relies on external commands that may fail for various reasons.
pub fn start_indexer_api(
    instance_id: Uuid,
    shutdown: CancellationToken,
    fut_docker: impl Future<Output = Result<Docker, ArcError>> + Clone + Send + 'static,
    fut_docker_network: impl Future<Output = Result<String, ArcError>> + Clone + Send + 'static,
    fut_postgres: impl Future<Output = Result<u16, ArcError>> + Clone + Send + 'static,
    fut_all_processors_ready: impl Future<Output = Result<(), ArcError>> + Clone + Send + 'static,
) -> (
    impl Future<Output = Result<u16>>,
    impl Future<Output = Result<()>>,
    impl Future<Output = ()>,
) {
    let fut_container_clean_up = Arc::new(Mutex::new(None));

    let fut_create_indexer_api = make_shared({
        let fut_docker = fut_docker.clone();
        let fut_container_clean_up = fut_container_clean_up.clone();

        async move {
            let (docker_network_name, _postgres_port, _) =
                try_join!(fut_docker_network, fut_postgres, fut_all_processors_ready).context(
                    "failed to start indexer api server: one or more dependencies failed to start",
                )?;

            no_panic_println!("Starting indexer API..");

            let (options, config) =
                create_container_options_and_config(instance_id, docker_network_name);
            let (fut_container, fut_container_cleanup) =
                create_start_and_inspect_container(shutdown.clone(), fut_docker, options, config);
            *fut_container_clean_up.lock().await = Some(fut_container_cleanup);

            let container_info = fut_container
                .await
                .context("failed to start indexer api server")?;

            let indexer_api_port = get_hasura_assigned_port(&container_info)
                .ok_or_else(|| anyhow!("failed to get indexer api server port"))?;

            anyhow::Ok(indexer_api_port)
        }
    });

    let fut_indexer_api_port = {
        let fut_create_indexer_api = fut_create_indexer_api.clone();

        async move {
            let indexer_api_port = fut_create_indexer_api.await?;

            let url =
                Url::parse(&format!("http://{}:{}", IP_LOCAL_HOST, indexer_api_port)).unwrap();

            // The first health checker waits for the service to be up at all.
            let health_checker = HealthChecker::Http(url.clone(), "Indexer API".to_string());
            health_checker
                .wait(None)
                .await
                .context("failed to wait for indexer API to be ready")?;

            no_panic_println!("Indexer API is up, applying hasura metadata..");

            // Apply the hasura metadata, with the second health checker waiting for it to succeed.
            post_metadata(url.clone(), HASURA_METADATA)
                .await
                .context("failed to apply hasura metadata")?;

            let health_checker_metadata = HealthChecker::IndexerApiMetadata(url);
            health_checker_metadata
                .wait(None)
                .await
                .context("failed to wait for indexer API to be ready")?;

            no_panic_println!(
                "Indexer API is ready. Endpoint: http://{}:{}/",
                IP_LOCAL_HOST,
                indexer_api_port
            );

            anyhow::Ok(indexer_api_port)
        }
    };

    let fut_indexer_api_finish = async move {
        let docker = fut_docker
            .await
            .context("failed to wait on indexer api container")?;

        // Wait for the container to stop (which it shouldn't).
        let _wait = docker
            .wait_container(
                &format!("velor-workspace-{}-indexer-api", instance_id),
                Some(WaitContainerOptions {
                    condition: "not-running",
                }),
            )
            .try_collect::<Vec<_>>()
            .await
            .context("failed to wait on indexer api container")?;

        anyhow::Ok(())
    };

    let fut_indexer_api_clean_up = {
        // Note: The creation task must be allowed to finish, even if a shutdown signal or other
        //       early abort signal is received. This is to prevent race conditions.
        //
        //       Do not abort the creation task prematurely -- let it either finish or handle its own abort.
        let fut_create_indexer_api = fut_create_indexer_api.clone();

        async move {
            _ = fut_create_indexer_api.await;

            if let Some(fut_container_clean_up) = fut_container_clean_up.lock().await.take() {
                fut_container_clean_up.await;
            }
        }
    };

    (
        fut_indexer_api_port,
        fut_indexer_api_finish,
        fut_indexer_api_clean_up,
    )
}
