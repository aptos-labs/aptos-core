// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::make_shared;
use anyhow::{anyhow, bail, Context, Result};
use aptos::node::local_testnet::docker;
use bollard::{
    container::{CreateContainerOptions, InspectContainerOptions, StartContainerOptions},
    network::CreateNetworkOptions,
    secret::ContainerInspectResponse,
    volume::CreateVolumeOptions,
};
use std::{future::Future, sync::Arc};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

pub fn create_docker_network(
    shutdown: CancellationToken,
    name: String,
) -> (
    impl Future<Output = Result<String, Arc<anyhow::Error>>> + Clone,
    impl Future<Output = ()>,
) {
    let needs_cleanup = Arc::new(Mutex::new(false));

    let fut_create_network = make_shared({
        let needs_cleanup = needs_cleanup.clone();
        let name = name.clone();

        let handle = tokio::spawn(async move {
            let docker = tokio::select! {
                _ = shutdown.cancelled() => {
                    bail!("failed to create docker network: cancelled")
                }
                res = docker::get_docker() => {
                    res.context("failed to create docker network")?
                }
            };

            *needs_cleanup.lock().await = true;

            docker
                .create_network(CreateNetworkOptions {
                    name: name.clone(),
                    internal: false,
                    check_duplicate: true,
                    ..Default::default()
                })
                .await
                .context("failed to create docker network")?;

            println!("Created docker network {}", name);

            Ok(name)
        });

        async move {
            handle
                .await
                .map_err(|err| anyhow!("failed to join task handle: {}", err))?
        }
    });

    let fut_clean_up = {
        let fut_create_network = fut_create_network.clone();

        async move {
            _ = fut_create_network.await;

            let network_name = name.as_str();
            let cleanup = async move {
                if *needs_cleanup.lock().await {
                    let docker = docker::get_docker().await?;
                    docker.remove_network(network_name).await?;
                }

                anyhow::Ok(())
            };

            match cleanup.await {
                Ok(_) => {
                    println!("Removed docker network {}", name);
                },
                Err(err) => {
                    eprintln!("Failed to remove docker network {}: {}", name, err)
                },
            }
        }
    };

    (fut_create_network, fut_clean_up)
}

pub fn create_docker_volume(
    shutdown: CancellationToken,
    name: String,
) -> (
    impl Future<Output = Result<String, Arc<anyhow::Error>>> + Clone,
    impl Future<Output = ()>,
) {
    let needs_cleanup = Arc::new(Mutex::new(false));

    let fut_create_volume = make_shared({
        let needs_cleanup = needs_cleanup.clone();
        let name = name.clone();

        let handle = tokio::spawn(async move {
            let docker = tokio::select! {
                _ = shutdown.cancelled() => {
                    bail!("failed to create docker volume: cancelled")
                }
                res = docker::get_docker() => {
                    res.context("failed to create docker volume")?
                }
            };

            *needs_cleanup.lock().await = true;

            docker
                .create_volume(CreateVolumeOptions {
                    name: name.clone(),
                    ..Default::default()
                })
                .await
                .context("failed to create docker volume")?;

            println!("Created docker volume {}", name);

            Ok(name)
        });

        async move {
            handle
                .await
                .map_err(|err| anyhow!("failed to join task handle: {}", err))?
        }
    });

    let fut_clean_up = {
        let fut_create_volume = fut_create_volume.clone();

        async move {
            _ = fut_create_volume.await;

            let volume_name = name.as_str();
            let cleanup = async move {
                if *needs_cleanup.lock().await {
                    let docker = docker::get_docker().await?;
                    docker.remove_volume(volume_name, None).await?;
                }

                anyhow::Ok(())
            };

            match cleanup.await {
                Ok(_) => {
                    println!("Removed docker volume {}", name);
                },
                Err(err) => {
                    eprintln!("Failed to remove docker volume {}: {}", name, err)
                },
            }
        }
    };

    (fut_create_volume, fut_clean_up)
}

pub fn create_start_and_inspect_container(
    shutdown: CancellationToken,
    options: CreateContainerOptions<String>,
    config: bollard::container::Config<String>,
) -> (
    impl Future<Output = Result<Arc<ContainerInspectResponse>, Arc<anyhow::Error>>> + Clone,
    impl Future<Output = ()>,
) {
    #[derive(PartialEq, Eq, Clone, Copy)]
    enum State {
        Initial = 0,
        Created = 1,
        Started = 2,
    }

    let state = Arc::new(Mutex::new(State::Initial));
    let name = options.name.clone();

    let fut_run = make_shared({
        let state = state.clone();
        let name = name.clone();

        let handle = tokio::spawn(async move {
            let docker = tokio::select! {
                _ = shutdown.cancelled() => {
                    bail!("failed to create docker container: cancelled")
                }
                res = docker::get_docker() => {
                    res.context("failed to create docker container")?
                }
            };

            let mut state = state.lock().await;

            *state = State::Created;
            docker
                .create_container(Some(options), config)
                .await
                .context("failed to create docker container")?
                .id;
            println!("Created docker container {}", name);

            if shutdown.is_cancelled() {
                bail!("failed to start docker container: cancelled")
            }
            *state = State::Started;
            docker
                .start_container(&name, None::<StartContainerOptions<&str>>)
                .await
                .context("failed to start docker container")?;
            println!("Started docker container {}", name);

            if shutdown.is_cancelled() {
                bail!("failed to inspect docker container: cancelled")
            }
            let container_info = docker
                .inspect_container(&name, Some(InspectContainerOptions::default()))
                .await
                .context("failed to inspect postgres container")?;

            Ok(Arc::new(container_info))
        });

        async move {
            handle
                .await
                .map_err(|err| anyhow!("failed to join task handle: {}", err))?
        }
    });

    let fut_clean_up = {
        let fut_run = fut_run.clone();

        async move {
            _ = fut_run.await;

            let state = state.lock().await;

            if *state == State::Initial {
                return;
            }

            let docker = match docker::get_docker().await {
                Ok(docker) => docker,
                Err(err) => {
                    eprintln!("Failed to clean up docker container {}: {}", name, err);
                    return;
                },
            };

            if *state == State::Started {
                match docker.stop_container(name.as_str(), None).await {
                    Ok(_) => {
                        println!("Stopped docker container {}", name)
                    },
                    Err(err) => {
                        eprintln!("Failed to stop docker container {}: {}", name, err)
                    },
                }
            }

            match docker.remove_container(name.as_str(), None).await {
                Ok(_) => {
                    println!("Removed docker container {}", name)
                },
                Err(err) => {
                    eprintln!("Failed to remove docker container {}: {}", name, err)
                },
            }
        }
    };

    (fut_run, fut_clean_up)
}
