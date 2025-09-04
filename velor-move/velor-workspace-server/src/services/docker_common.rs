// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{make_shared, ArcError},
    no_panic_eprintln, no_panic_println,
};
use anyhow::{anyhow, bail, Context, Result};
use bollard::{
    container::{CreateContainerOptions, InspectContainerOptions, StartContainerOptions},
    image::CreateImageOptions,
    network::CreateNetworkOptions,
    secret::ContainerInspectResponse,
    volume::CreateVolumeOptions,
    Docker,
};
use futures::TryStreamExt;
use std::{future::Future, sync::Arc};
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

/// Creates a permanent docker network which does not need to be cleaned up.
pub async fn create_docker_network_permanent(
    shutdown: CancellationToken,
    fut_docker: impl Future<Output = Result<Docker, ArcError>> + Clone + Send + 'static,
    name: String,
) -> Result<String, anyhow::Error> {
    let handle = tokio::spawn(async move {
        let docker = tokio::select! {
            _ = shutdown.cancelled() => {
                bail!("failed to create docker network: cancelled")
            }
            res = fut_docker => {
                res.context("failed to create docker network")?
            }
        };

        let res = docker
            .create_network(CreateNetworkOptions {
                name: name.clone(),
                internal: false,
                check_duplicate: true,
                ..Default::default()
            })
            .await;

        match res {
            Ok(_response) => {
                no_panic_println!("Created docker network {}", name);

                Ok(name)
            },
            Err(err) => match err {
                bollard::errors::Error::DockerResponseServerError {
                    status_code: 409, ..
                } => {
                    no_panic_println!("Docker network {} already exists, not creating it", name);
                    Ok(name)
                },
                err => Err(err.into()),
            },
        }
    });

    handle
        .await
        .map_err(|err| anyhow!("failed to join task handle: {}", err))?
}

/// Creates a Docker network asynchronously and provides a cleanup task for network removal.
///
/// A cancellation token can be used to signal an early shutdown, allowing the creation task to
/// abort without performing any additional steps.
///
/// It returns two futures
/// 1. One that creates the Docker network
/// 2. Another that handles the cleanup (removal of the network)
///
/// As the caller, you should always await the cleanup task when you are ready to shutdown the
/// service. The cleanup task will only attempt to remove the network if it may have been created.
///
/// Note that the cleanup is a "best-effort" operation -- success is not guaranteed due to
/// reliance on external commands, which may fail for various reasons.
#[allow(unused)]
pub fn create_docker_network(
    shutdown: CancellationToken,
    fut_docker: impl Future<Output = Result<Docker, ArcError>> + Clone + Send + 'static,
    name: String,
) -> (
    impl Future<Output = Result<String, ArcError>> + Clone,
    impl Future<Output = ()>,
) {
    // Flag indicating whether cleanup is needed.
    //
    // Note: The `Arc<Mutex<..>>` is used to satisfy Rust's borrow checking rules.
    //       Exclusive access is ensured by the sequencing of the futures.
    let needs_cleanup = Arc::new(Mutex::new(false));

    let fut_create_network = make_shared({
        let needs_cleanup = needs_cleanup.clone();
        let name = name.clone();
        let fut_docker = fut_docker.clone();

        let handle = tokio::spawn(async move {
            let docker = tokio::select! {
                _ = shutdown.cancelled() => {
                    bail!("failed to create docker network: cancelled")
                }
                res = fut_docker => {
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

            no_panic_println!("Created docker network {}", name);

            Ok(name)
        });

        async move {
            handle
                .await
                .map_err(|err| anyhow!("failed to join task handle: {}", err))?
        }
    });

    let fut_clean_up = {
        // Note: The creation task must be allowed to finish, even if a shutdown signal or other
        //       early abort signal is received. This is to prevent race conditions.
        //
        //       Do not abort the creation task prematurely -- let it either finish or handle its own abort.
        let fut_create_network = fut_create_network.clone();

        async move {
            _ = fut_create_network.await;

            let network_name = name.as_str();
            let cleanup = async move {
                if *needs_cleanup.lock().await {
                    let docker = fut_docker.await?;
                    docker.remove_network(network_name).await?;
                }

                anyhow::Ok(())
            };

            match cleanup.await {
                Ok(_) => {
                    no_panic_println!("Removed docker network {}", name);
                },
                Err(err) => {
                    no_panic_eprintln!("Failed to remove docker network {}: {}", name, err)
                },
            }
        }
    };

    (fut_create_network, fut_clean_up)
}

/// Creates a Docker volume asynchronously and provides a cleanup task for volume removal.
///
/// A cancellation token can be used to signal an early shutdown, allowing the creation task to
/// abort without performing any additional steps.
///
/// It returns two futures
/// 1. One that creates the Docker volume
/// 2. Another that handles the cleanup (removal of the volume)
///
/// As the caller, you should always await the cleanup task when you are ready to shutdown the
/// service. The cleanup task will only attempt to remove the volume if it may have been created.
///
/// Note that the cleanup is a "best-effort" operation -- success is not guaranteed due to
/// success is not guaranteed due to the reliance on external commands, which may fail for
/// various reasons.
pub fn create_docker_volume(
    shutdown: CancellationToken,
    fut_docker: impl Future<Output = Result<Docker, ArcError>> + Clone + Send + 'static,
    name: String,
) -> (
    impl Future<Output = Result<String, ArcError>> + Clone,
    impl Future<Output = ()>,
) {
    // Flag indicating whether cleanup is needed.
    //
    // Note: The `Arc<Mutex<..>>` is used to satisfy Rust's borrow checking rules.
    //       Exclusive access is ensured by the sequencing of the futures.
    let needs_cleanup = Arc::new(Mutex::new(false));

    let fut_create_volume = make_shared({
        let needs_cleanup = needs_cleanup.clone();
        let name = name.clone();
        let fut_docker = fut_docker.clone();

        let handle = tokio::spawn(async move {
            let docker = tokio::select! {
                _ = shutdown.cancelled() => {
                    bail!("failed to create docker volume: cancelled")
                }
                res = fut_docker => {
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

            no_panic_println!("Created docker volume {}", name);

            Ok(name)
        });

        async move {
            handle
                .await
                .map_err(|err| anyhow!("failed to join task handle: {}", err))?
        }
    });

    let fut_clean_up = {
        // Note: The creation task must be allowed to finish, even if a shutdown signal or other
        //       early abort signal is received. This is to prevent race conditions.
        //
        //       Do not abort the creation task prematurely -- let it either finish or handle its own abort.
        let fut_create_volume = fut_create_volume.clone();

        async move {
            _ = fut_create_volume.await;

            let volume_name = name.as_str();
            let cleanup = async move {
                if *needs_cleanup.lock().await {
                    let docker = fut_docker.await?;
                    docker.remove_volume(volume_name, None).await?;
                }

                anyhow::Ok(())
            };

            match cleanup.await {
                Ok(_) => {
                    no_panic_println!("Removed docker volume {}", name);
                },
                Err(err) => {
                    no_panic_eprintln!("Failed to remove docker volume {}: {}", name, err)
                },
            }
        }
    };

    (fut_create_volume, fut_clean_up)
}

/// Creates, starts, and inspects a Docker container asynchronously, and provides a cleanup task
/// for stopping and removing the container.
///
/// A cancellation token can be used to signal an early shutdown, allowing the creation task to
/// abort without performing any additional steps.
///
/// It returns two futures
/// 1. One that creates the container, starts it, and inspects it.
///    It resolves with the container's inspection info, such as port binding.
/// 2. Another that handles the cleanup (stopping and removing the container)
///
/// As the caller, you should always await the cleanup task when you are ready to shutdown the
/// service. The cleanup task will only attempt to stop or remove the container if it may have
/// gotten past the respective states.
///
/// Note that the cleanup is a "best-effort" operation -- success is not guaranteed due to
/// success is not guaranteed due to the reliance on external commands, which may fail for
/// various reasons.
pub fn create_start_and_inspect_container(
    shutdown: CancellationToken,
    fut_docker: impl Future<Output = Result<Docker, ArcError>> + Clone + Send + 'static,
    options: CreateContainerOptions<String>,
    config: bollard::container::Config<String>,
) -> (
    impl Future<Output = Result<Arc<ContainerInspectResponse>, ArcError>> + Clone,
    impl Future<Output = ()>,
) {
    #[derive(PartialEq, Eq, Clone, Copy)]
    enum State {
        Initial = 0,
        Created = 1,
        Started = 2,
    }

    // Flag indicating the current stage of the creation task and which resources need
    // to be cleaned up.
    //
    // Note: The `Arc<Mutex<..>>` is used to satisfy Rust's borrow checking rules.
    //       Exclusive access is ensured by the sequencing of the futures.
    let state = Arc::new(Mutex::new(State::Initial));
    let name = options.name.clone();

    let fut_run = make_shared({
        let state = state.clone();
        let name = name.clone();
        let fut_docker = fut_docker.clone();

        let handle = tokio::spawn(async move {
            let docker = tokio::select! {
                _ = shutdown.cancelled() => {
                    bail!("failed to create docker container: cancelled")
                }
                res = fut_docker => {
                    res.context("failed to create docker container")?
                }
            };

            let image_name = config.image.as_ref().unwrap();
            match docker.inspect_image(image_name).await {
                Ok(_) => {
                    no_panic_println!("Docker image {} already exists", image_name);
                },
                Err(_err) => {
                    no_panic_println!(
                        "Docker image {} does not exist. Pulling image..",
                        image_name
                    );

                    docker
                        .create_image(
                            Some(CreateImageOptions {
                                from_image: image_name.clone(),
                                ..Default::default()
                            }),
                            None,
                            None,
                        )
                        .try_collect::<Vec<_>>()
                        .await
                        .context("failed to create docker container")?;

                    no_panic_println!("Pulled docker image {}", image_name);
                },
            }

            let mut state = state.lock().await;

            *state = State::Created;
            docker
                .create_container(Some(options), config)
                .await
                .context("failed to create docker container")?;
            no_panic_println!("Created docker container {}", name);

            if shutdown.is_cancelled() {
                bail!("failed to start docker container: cancelled")
            }
            *state = State::Started;
            docker
                .start_container(&name, None::<StartContainerOptions<&str>>)
                .await
                .context("failed to start docker container")?;
            no_panic_println!("Started docker container {}", name);

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
            // Note: The creation task must be allowed to finish, even if a shutdown signal or other
            //       early abort signal is received. This is to prevent race conditions.
            //
            //       Do not abort the creation task prematurely -- let it either finish or handle its own abort.
            _ = fut_run.await;

            let state = state.lock().await;

            if *state == State::Initial {
                return;
            }

            let docker = match fut_docker.await {
                Ok(docker) => docker,
                Err(err) => {
                    no_panic_eprintln!("Failed to clean up docker container {}: {}", name, err);
                    return;
                },
            };

            if *state == State::Started {
                match docker.stop_container(name.as_str(), None).await {
                    Ok(_) => {
                        no_panic_println!("Stopped docker container {}", name)
                    },
                    Err(err) => {
                        no_panic_eprintln!("Failed to stop docker container {}: {}", name, err)
                    },
                }
            }

            match docker.remove_container(name.as_str(), None).await {
                Ok(_) => {
                    no_panic_println!("Removed docker container {}", name)
                },
                Err(err) => {
                    no_panic_eprintln!("Failed to remove docker container {}: {}", name, err)
                },
            }
        }
    };

    (fut_run, fut_clean_up)
}
