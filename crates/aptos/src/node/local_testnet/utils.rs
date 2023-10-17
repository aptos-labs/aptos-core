// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::traits::ShutdownStep;
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::{
    container::{RemoveContainerOptions, StopContainerOptions},
    image::CreateImageOptions,
    Docker,
};
use futures::TryStreamExt;
use reqwest::Url;
use std::{fs::create_dir_all, net::SocketAddr, path::Path};
use tracing::info;
use version_compare::Version;

pub fn socket_addr_to_url(socket_addr: &SocketAddr, scheme: &str) -> Result<Url> {
    let host = match socket_addr {
        SocketAddr::V4(v4) => format!("{}", v4.ip()),
        SocketAddr::V6(v6) => format!("[{}]", v6.ip()),
    };
    let full_url = format!("{}://{}:{}", scheme, host, socket_addr.port());
    Ok(Url::parse(&full_url)?)
}

pub fn get_docker() -> Result<Docker> {
    let docker = Docker::connect_with_local_defaults()
        .context("Docker is not available, confirm it is installed and running. On Linux you may need to use sudo.")?;
    Ok(docker)
}

pub async fn confirm_docker_available() -> Result<()> {
    let docker = get_docker()?;
    let info = docker
        .info()
        .await
        .context("Docker is not available, confirm it is installed and running. On Linux you may need to use sudo.")?;

    info!("Docker Info: {:?}", info);

    let version = docker
        .version()
        .await
        .context("Failed to get Docker version")?;

    info!("Docker Version: {:?}", version);

    // Try to warn the user about their Docker version being too old. We don't error
    // out if the version is too old in case we're wrong about the minimum version
    // for their particular system. We just print a warning.
    match version.api_version {
        Some(current_api_version) => match Version::from(&current_api_version) {
            Some(current_api_version) => {
                let minimum_api_version = Version::from("1.42").unwrap();
                if current_api_version < minimum_api_version {
                    eprintln!(
                            "WARNING: Docker API version {} is too old, minimum required version is {}. Please update Docker!",
                            current_api_version,
                            minimum_api_version,
                        );
                } else {
                    eprintln!("Docker version is sufficient: {}", current_api_version);
                }
            },
            None => {
                eprintln!(
                    "WARNING: Failed to parse Docker API version: {}",
                    current_api_version
                );
            },
        },
        None => {
            eprintln!(
                "WARNING: Failed to determine Docker version, confirm your Docker is up to date!"
            );
        },
    }

    Ok(())
}

/// Delete a container. If the container doesn't exist, that's fine, just move on.
pub async fn delete_container(container_name: &str) -> Result<()> {
    info!(
        "Removing container with name {} (if it exists)",
        container_name
    );

    let docker = get_docker()?;

    let options = Some(RemoveContainerOptions {
        force: true,
        ..Default::default()
    });

    // Ignore any error, it'll be because the container doesn't exist.
    let _ = docker.remove_container(container_name, options).await;

    info!(
        "Removed container with name {} (if it existed)",
        container_name
    );

    Ok(())
}

/// Stop a container. If the container doesn't exist, that's fine, just move on.
pub async fn stop_container(container_name: &str) -> Result<()> {
    info!(
        "Stopping container with name {} (if it exists)",
        container_name
    );

    let docker = get_docker()?;

    let options = Some(StopContainerOptions {
        // Timeout in seconds before we kill the contianer.
        t: 1,
    });

    // Ignore any error, it'll be because the container doesn't exist.
    let _ = docker.stop_container(container_name, options).await;

    info!(
        "Stopping container with name {} (if it existed)",
        container_name
    );

    Ok(())
}

pub async fn pull_docker_image(image_name: &str) -> Result<()> {
    info!("Pulling docker image {}", image_name);

    let docker = get_docker()?;

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
/// When the local testnet is started again, any leftover container will be deleted.
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
