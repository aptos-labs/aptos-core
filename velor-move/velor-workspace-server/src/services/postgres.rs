// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{make_shared, ArcError, IP_LOCAL_HOST},
    no_panic_println,
    services::docker_common::{create_docker_volume, create_start_and_inspect_container},
};
use anyhow::{anyhow, Context, Result};
use velor_localnet::health_checker::HealthChecker;
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
use uuid::Uuid;

const POSTGRES_DEFAULT_PORT: u16 = 5432;
const POSTGRES_IMAGE: &str = "postgres:14.11";
const POSTGRES_USER: &str = "postgres";
const POSTGRES_DB_NAME: &str = "local-testnet";

/// Extracts the host port assigned to the postgres container from its inspection data.
fn get_postgres_assigned_port(container_info: &ContainerInspectResponse) -> Option<u16> {
    if let Some(port_bindings) = container_info
        .network_settings
        .as_ref()
        .and_then(|ns| ns.ports.as_ref())
    {
        if let Some(Some(bindings)) = port_bindings.get(&format!("{}/tcp", POSTGRES_DEFAULT_PORT)) {
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

/// Constructs a connection string for accessing the postgres database from the host.
///
/// Note: This connection string is intended for use on the host only.
///       If you want to access the database from within another docker container,
///       use [`get_postgres_connection_string_within_docker_network`] instead.
pub fn get_postgres_connection_string(postgres_port: u16) -> String {
    format!(
        "postgres://{}@{}:{}/{}",
        POSTGRES_USER, IP_LOCAL_HOST, postgres_port, POSTGRES_DB_NAME
    )
}

/// Constructs a connection string for accessing the postgres database from within
/// another docker container.
///
/// Note: This connection string is intended for use within a docker network only.
///       If you want to access the database from clients running on the host directly,
///       use [`get_postgres_connection_string`] instead.
pub fn get_postgres_connection_string_within_docker_network(instance_id: Uuid) -> String {
    format!(
        "postgres://{}@velor-workspace-{}-postgres:{}/{}",
        POSTGRES_USER, instance_id, POSTGRES_DEFAULT_PORT, POSTGRES_DB_NAME
    )
}

/// Returns the Docker container options and configuration to start a postgres container with
/// - The container bound to a Docker network (`network_name`).
/// - [`POSTGRES_DEFAULT_PORT`] mapped to a random OS-assigned host port.
/// - A volume (`volume_name`) mounted for data storage.
/// - Environment variables to configure postgres with
///   - Authentication method: trust
///   - User: [`POSTGRES_USER`],
///   - Database: [`POSTGRES_DB_NAME`].
fn create_container_options_and_config(
    instance_id: Uuid,
    network_name: String,
    volume_name: String,
) -> (
    CreateContainerOptions<String>,
    bollard::container::Config<String>,
) {
    let host_config = Some(HostConfig {
        // Bind the container to the network we created in the pre_run. This does
        // not prevent the binary in the container from exposing itself to the host
        // on 127.0.0.1. See more here: https://stackoverflow.com/a/77432636/3846032.
        network_mode: Some(network_name.clone()),
        port_bindings: Some(hashmap! {
            POSTGRES_DEFAULT_PORT.to_string() => Some(vec![PortBinding {
                host_ip: Some(IP_LOCAL_HOST.to_string()),
                host_port: None,
            }]),
        }),
        // Mount the volume in to the container. We use a volume because they are
        // more performant and easier to manage via the Docker API.
        binds: Some(vec![format!("{}:/var/lib/postgresql/data", volume_name)]),
        ..Default::default()
    });

    let config = bollard::container::Config {
        image: Some(POSTGRES_IMAGE.to_string()),
        // We set this to false so the container keeps running after the CLI
        // shuts down by default. We manually kill the container if applicable,
        // for example if the user set --force-restart.
        tty: Some(false),
        exposed_ports: Some(hashmap! {POSTGRES_DEFAULT_PORT.to_string() => hashmap!{}}),
        host_config,
        env: Some(vec![
            // We run postgres without any auth + no password.
            "POSTGRES_HOST_AUTH_METHOD=trust".to_string(),
            format!("POSTGRES_USER={}", POSTGRES_USER),
            format!("POSTGRES_DB={}", POSTGRES_DB_NAME),
            // This tells where postgres to store the DB data on disk. This is the
            // directory inside the container that is mounted from the host system.
            // format!("PGDATA={}", DATA_PATH_IN_CONTAINER),
        ]),
        cmd: Some(
            vec![
                "postgres",
                "-c",
                // The default is 100 as of Postgres 14.11. Given the localnet
                // can be composed of many different processors all with their own
                // connection pools, 100 is insufficient.
                "max_connections=200",
                "-c",
                // The default is 128MB as of Postgres 14.11. We 2x that value to
                // match the fact that we 2x'd max_connections.
                "shared_buffers=256MB",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
        ),
        ..Default::default()
    };

    let options = CreateContainerOptions {
        name: format!("velor-workspace-{}-postgres", instance_id),
        ..Default::default()
    };

    (options, config)
}

/// Starts a postgres container within a docker network.
///
/// Prerequisites
/// - This depends on a previous task to create the docker network
///
/// The function returns three futures:
/// - One that resolves to the host port that can be used to access the postgres container when
///   it's fully up.
/// - One that resolves when the container stops (which it should not under normal operation).
/// - A cleanup task that stops the container and removes the associated data volume.
///
/// As the caller, you should always await the cleanup task when you are ready to shutdown the
/// service. The cleanup is a "best-effort" operation -- success is not guaranteed
/// as it relies on external commands that may fail for various reasons.
pub fn start_postgres(
    shutdown: CancellationToken,
    fut_docker: impl Future<Output = Result<Docker, ArcError>> + Clone + Send + 'static,
    fut_network: impl Future<Output = Result<String, ArcError>>,
    instance_id: Uuid,
) -> (
    impl Future<Output = Result<u16>>,
    impl Future<Output = Result<()>>,
    impl Future<Output = ()>,
) {
    no_panic_println!("Starting postgres..");

    let volume_name = format!("velor-workspace-{}-postgres", instance_id);
    let (fut_volume, fut_volume_clean_up) =
        create_docker_volume(shutdown.clone(), fut_docker.clone(), volume_name);

    let fut_container_clean_up = Arc::new(Mutex::new(None));

    let fut_create_postgres = make_shared({
        let fut_container_clean_up = fut_container_clean_up.clone();
        let fut_docker = fut_docker.clone();

        async move {
            let (network_name, volume_name) = try_join!(fut_network, fut_volume)
                .context("failed to start postgres: one or more dependencies failed to start")?;

            let (options, config) =
                create_container_options_and_config(instance_id, network_name, volume_name);
            let (fut_container, fut_container_cleanup) =
                create_start_and_inspect_container(shutdown.clone(), fut_docker, options, config);
            *fut_container_clean_up.lock().await = Some(fut_container_cleanup);

            let container_info = fut_container.await.context("failed to start postgres")?;

            let postgres_port = get_postgres_assigned_port(&container_info)
                .ok_or_else(|| anyhow!("failed to get postgres port"))?;

            anyhow::Ok(postgres_port)
        }
    });

    let fut_postgres_port = {
        let fut_create_postgres = fut_create_postgres.clone();

        async move {
            let postgres_port = fut_create_postgres.await?;

            let health_checker =
                HealthChecker::Postgres(get_postgres_connection_string(postgres_port));
            health_checker.wait(None).await?;

            no_panic_println!(
                "Postgres is ready. Endpoint: http://{}:{}",
                IP_LOCAL_HOST,
                postgres_port
            );

            anyhow::Ok(postgres_port)
        }
    };

    let fut_postgres_finish = async move {
        let docker = fut_docker
            .await
            .context("failed to wait on postgres container")?;

        // Wait for the container to stop (which it shouldn't).
        let _wait = docker
            .wait_container(
                &format!("velor-workspace-{}-postgres", instance_id),
                Some(WaitContainerOptions {
                    condition: "not-running",
                }),
            )
            .try_collect::<Vec<_>>()
            .await
            .context("failed to wait on postgres container")?;

        anyhow::Ok(())
    };

    let fut_postgres_clean_up = {
        // Note: The creation task must be allowed to finish, even if a shutdown signal or other
        //       early abort signal is received. This is to prevent race conditions.
        //
        //       Do not abort the creation task prematurely -- let it either finish or handle its own abort.
        let fut_create_postgres = fut_create_postgres.clone();

        async move {
            _ = fut_create_postgres.await;

            if let Some(fut_container_clean_up) = fut_container_clean_up.lock().await.take() {
                fut_container_clean_up.await;
            }
            fut_volume_clean_up.await;
        }
    };

    (
        fut_postgres_port,
        fut_postgres_finish,
        fut_postgres_clean_up,
    )
}
