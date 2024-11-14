// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::IP_LOCAL_HOST;
use anyhow::{anyhow, bail, Context, Result};
use aptos::node::local_testnet::{docker, HealthChecker};
use bollard::{
    container::{
        CreateContainerOptions, InspectContainerOptions, StartContainerOptions,
        WaitContainerOptions,
    },
    network::CreateNetworkOptions,
    secret::{ContainerInspectResponse, HostConfig, PortBinding},
};
use futures::{channel::oneshot, TryStreamExt};
use maplit::hashmap;
use std::{future::Future, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;

const POSTGRES_DEFAULT_PORT: u16 = 5432;
const POSTGRES_IMAGE: &str = "postgres:14.11";
const POSTGRES_USER: &str = "postgres";
const POSTGRES_DB_NAME: &str = "local-testnet";

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

enum PostgresState {
    Stopped,
    Active {
        data_volume: Option<String>,
        network: Option<String>,
        container: Option<String>,
    },
}

pub fn get_postgres_connection_string(postgres_port: u16) -> String {
    format!(
        "postgres://{}@{}:{}/{}",
        POSTGRES_USER, IP_LOCAL_HOST, postgres_port, POSTGRES_DB_NAME
    )
}

pub fn start_postgres(
    instance_id: Uuid,
) -> Result<(
    impl Future<Output = Result<u16>>,
    impl Future<Output = Result<()>>,
    impl Future<Output = Result<()>>,
)> {
    use PostgresState::*;

    let (postgres_container_id_tx, postgres_container_id_rx) = oneshot::channel();

    let state_ = Arc::new(Mutex::new(PostgresState::Active {
        data_volume: None,
        network: None,
        container: None,
    }));

    let state = state_.clone();
    let handle_postgres = tokio::spawn(async move {
        println!("Starting postgres..");

        let docker = docker::get_docker().await?;

        let data_volume_name = format!("aptos-workspace-{}-postgres-data", instance_id);
        {
            let mut guard = state.lock().await;
            match &mut *guard {
                Active { data_volume, .. } => {
                    let vol = docker
                        .create_volume(bollard::volume::CreateVolumeOptions {
                            name: data_volume_name.as_str(),
                            ..Default::default()
                        })
                        .await
                        .context("failed to create data volume for postgres")?;
                    *data_volume = Some(vol.name.clone())
                },
                Stopped => bail!("cancellation requested"),
            }
        }
        println!("Created docker volume {}", data_volume_name);

        let network_name = format!("aptos-workspace-{}-postgres", instance_id);
        {
            let mut guard = state.lock().await;
            match &mut *guard {
                Active { network, .. } => {
                    docker
                        .create_network(CreateNetworkOptions {
                            name: network_name.as_str(),
                            internal: false,
                            check_duplicate: true,
                            ..Default::default()
                        })
                        .await
                        .context("failed to create network for postgres")?;
                    *network = Some(network_name.clone())
                },
                Stopped => bail!("cancellation requested"),
            }
        }
        println!("Created docker network {}", network_name);

        let host_config = Some(HostConfig {
            // Bind the container to the network we created in the pre_run. This does
            // not prevent the binary in the container from exposing itself to the host
            // on 127.0.0.1. See more here: https://stackoverflow.com/a/77432636/3846032.
            network_mode: Some(network_name.clone()),
            port_bindings: Some(hashmap! {
                POSTGRES_DEFAULT_PORT.to_string() => Some(vec![PortBinding {
                    host_ip: Some("127.0.0.1".to_string()),
                    host_port: None,
                }]),
            }),
            // Mount the volume in to the container. We use a volume because they are
            // more performant and easier to manage via the Docker API.
            binds: Some(vec![format!(
                "{}:/var/lib/postgresql/data",
                data_volume_name,
            )]),
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

        let options = Some(CreateContainerOptions {
            name: format!("aptos-workspace-{}-postgres", instance_id),
            ..Default::default()
        });

        let (container_id, container_info) = {
            let mut guard = state.lock().await;
            match &mut *guard {
                Active { container, .. } => {
                    let container_id = docker
                        .create_container(options, config)
                        .await
                        .context("failed to create postgres container")?
                        .id;

                    docker
                        .start_container(&container_id, None::<StartContainerOptions<&str>>)
                        .await
                        .context("failed to start postgres container")?;

                    let container_info = docker
                        .inspect_container(&container_id, Some(InspectContainerOptions::default()))
                        .await
                        .context("failed to inspect postgres container")?;

                    *container = Some(container_id.clone());

                    (container_id, container_info)
                },
                Stopped => bail!("cancellation requested"),
            }
        };
        println!("Created docker container {}", container_id);

        postgres_container_id_tx
            .send(container_id)
            .map_err(|_port| anyhow!("failed to send postgres container id"))?;

        let postgres_port = get_postgres_assigned_port(&container_info)
            .ok_or_else(|| anyhow!("failed to get postgres port"))?;

        // TODO: health checker
        let health_checker = HealthChecker::Postgres(get_postgres_connection_string(postgres_port));
        health_checker.wait(None).await?;

        println!(
            "Postgres is ready. Endpoint: http://{}:{}",
            IP_LOCAL_HOST, postgres_port
        );

        Ok(postgres_port)
    });

    //let abort_handle = handle_postgres.abort_handle();

    let fut_postgres_port = async move {
        handle_postgres
            .await
            .map_err(|err| anyhow!("failed to join handle task: {}", err))?
    };

    let state = state_.clone();
    let fut_postgres_cancel = async move {
        let mut guard = state.lock().await;

        match &*guard {
            Active {
                data_volume,
                network,
                container,
            } => {
                let docker = docker::get_docker().await?;

                if let Some(container) = container {
                    docker.stop_container(container.as_str(), None).await?;
                    println!("Stopped docker container {}", container);
                    docker.remove_container(container.as_str(), None).await?;
                    println!("Removed docker container {}", container);
                }
                if let Some(network) = network {
                    docker.remove_network(network.as_str()).await?;
                    println!("Removed docker network {}", network);
                }
                if let Some(volume) = data_volume {
                    docker.remove_volume(volume.as_str(), None).await?;
                    println!("Removed docker volume {}", volume);
                }

                *guard = PostgresState::Stopped;
                //abort_handle.abort();

                Ok(())
            },
            Stopped => Ok(()),
        }
    };

    let fut_postgres_finish = async move {
        let container_id = postgres_container_id_rx
            .await
            .context("failed to receive postgres container id")?;

        let docker = docker::get_docker().await?;

        // Wait for the container to stop (which it shouldn't).
        let _wait = docker
            .wait_container(
                &container_id,
                Some(WaitContainerOptions {
                    condition: "not-running",
                }),
            )
            .try_collect::<Vec<_>>()
            .await
            .context("Failed to wait on postgres container")?;

        Ok(())
    };

    Ok((fut_postgres_port, fut_postgres_finish, fut_postgres_cancel))
}
