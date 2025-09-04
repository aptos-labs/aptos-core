// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::traits::ShutdownStep;
use anyhow::{Context, Result};
pub use velor_localnet::docker::get_docker;
use async_trait::async_trait;
use bollard::{
    container::{RemoveContainerOptions, StopContainerOptions},
    errors::Error as BollardError,
    image::CreateImageOptions,
    network::CreateNetworkOptions,
    volume::{CreateVolumeOptions, RemoveVolumeOptions},
};
use futures::TryStreamExt;
use std::{fs::create_dir_all, path::Path};
use tracing::{info, warn};

pub const CONTAINER_NETWORK_NAME: &str = "velor-local-testnet-network";

/// Delete a container. If the container doesn't exist, that's fine, just move on.
pub async fn delete_container(container_name: &str) -> Result<()> {
    info!(
        "Removing container with name {} (if it exists)",
        container_name
    );

    let docker = get_docker().await?;

    let options = Some(RemoveContainerOptions {
        force: true,
        ..Default::default()
    });

    // Ignore any error, it'll be because the container doesn't exist.
    let result = docker.remove_container(container_name, options).await;

    match result {
        Ok(_) => info!("Successfully removed container {}", container_name),
        Err(err) => warn!(
            "Failed to remove container {}: {:#} (it probably didn't exist)",
            container_name, err
        ),
    }

    Ok(())
}

/// Stop a container. If the container doesn't exist, that's fine, just move on.
pub async fn stop_container(container_name: &str) -> Result<()> {
    info!(
        "Stopping container with name {} (if it exists)",
        container_name
    );

    let docker = get_docker().await?;

    let options = Some(StopContainerOptions {
        // Timeout in seconds before we kill the container.
        t: 1,
    });

    // Ignore any error, it'll be because the container doesn't exist.
    let result = docker.stop_container(container_name, options).await;

    match result {
        Ok(_) => info!("Successfully stopped container {}", container_name),
        Err(err) => warn!(
            "Failed to stop container {}: {:#} (it probably didn't exist)",
            container_name, err
        ),
    }

    Ok(())
}

pub async fn pull_docker_image(image_name: &str) -> Result<()> {
    info!("Checking if we have to pull docker image {}", image_name);

    let docker = get_docker().await?;

    let options = Some(CreateImageOptions {
        from_image: image_name,
        ..Default::default()
    });

    // Check if the image is there. If it is, exit early, the user can update any
    // images we've already pulled manually if they want.
    if docker.inspect_image(image_name).await.is_ok() {
        info!(
            "Image {} found locally, not attempting to pull it",
            image_name
        );
        return Ok(());
    }

    // The image is not present, let the user know we'll pull it.
    eprintln!("Image {} not found, pulling it now...", image_name);

    // The docker pull CLI command is just sugar around this API.
    docker
        .create_image(options, None, None)
        // Just wait for the whole stream, we don't need to do other things in parallel.
        .try_collect::<Vec<_>>()
        .await
        .with_context(|| format!("Failed to pull image {}", image_name))?;

    info!("Pulled docker image {}", image_name);

    Ok(())
}

/// Create a network. If the network already exists, that's fine, just move on.
pub async fn create_network(network_name: &str) -> Result<()> {
    let docker = get_docker().await?;

    info!("Creating network {}", network_name);

    let config = CreateNetworkOptions {
        name: network_name,
        internal: false,
        check_duplicate: true,
        ..Default::default()
    };
    let response = docker.create_network(config).await;

    match response {
        Ok(_) => {
            info!("Created volume {}", network_name);
            Ok(())
        },
        Err(err) => match err {
            BollardError::DockerResponseServerError { status_code, .. } => {
                if status_code == 409 {
                    info!("Network {} already exists, not creating it", network_name);
                    Ok(())
                } else {
                    Err(err.into())
                }
            },
            wildcard => Err(wildcard.into()),
        },
    }
}

pub async fn create_volume(volume_name: &str) -> Result<()> {
    let docker = get_docker().await?;

    info!("Creating volume {}", volume_name);

    let config = CreateVolumeOptions {
        name: volume_name,
        ..Default::default()
    };
    docker.create_volume(config).await?;

    info!("Created volume {}", volume_name);

    Ok(())
}

pub async fn delete_volume(volume_name: &str) -> Result<()> {
    let docker = get_docker().await?;

    info!("Removing volume {}", volume_name);

    let config = RemoveVolumeOptions { force: true };

    // Delete the volume. This returns Ok even if the volume didn't exist, unlike the
    // other "remove_x" endpoints, so we just use ? here.
    docker
        .remove_volume(volume_name, Some(config))
        .await
        .context(format!("Failed to remove volume {}", volume_name))?;

    info!(
        "Successfully removed volume {} (if it existed in the first place)",
        volume_name
    );

    Ok(())
}

/// This function creates a directory called `dir_name` under `test_dir` and writes a
/// file called README.md that tells the user where to go to see logs. We do this since
/// having the user use `docker logs` is the preferred approach, rather than writing
/// logs to files (which is complex and can slow down the container).
pub fn setup_docker_logging(test_dir: &Path, dir_name: &str, container_name: &str) -> Result<()> {
    // Create dir.
    let log_dir = test_dir.join(dir_name);
    create_dir_all(log_dir.as_path()).context(format!("Failed to create {}", log_dir.display()))?;

    // Write README.
    let data = format!(
        "To see logs for {} run the following command:\n\ndocker logs {}\n",
        dir_name, container_name
    );
    std::fs::write(log_dir.join("README.md"), data).context("Unable to write README file")?;

    Ok(())
}

/// This shutdown step stops a container with the given name. If no container is found
/// we continue without error. We choose to stop the container on shutdown rather than
/// totally delete it so the user can check the logs if it was an unexpected shutdown.
/// When the localnet is started again, any leftover container will be deleted.
#[derive(Clone, Debug)]
pub struct StopContainerShutdownStep {
    container_name: &'static str,
}

impl StopContainerShutdownStep {
    pub fn new(container_name: &'static str) -> Self {
        Self { container_name }
    }
}

#[async_trait]
impl ShutdownStep for StopContainerShutdownStep {
    async fn run(self: Box<Self>) -> Result<()> {
        stop_container(self.container_name).await?;
        Ok(())
    }
}
