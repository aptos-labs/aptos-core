// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::traits::ShutdownStep;
use anyhow::{Context, Result};
use async_trait::async_trait;
use bollard::{container::RemoveContainerOptions, image::CreateImageOptions, Docker};
use futures::TryStreamExt;
use reqwest::Url;
use std::{fs::create_dir_all, net::SocketAddr, path::Path};
use tracing::info;

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
        .context("Docker is not available, confirm it is installed and running: {:?}")?;
    Ok(docker)
}

pub async fn confirm_docker_available() -> Result<()> {
    let docker = get_docker()?;
    docker
        .info()
        .await
        .context("Docker is not available, confirm it is installed and running: {:?}")?;
    Ok(())
}

/// Delete a container. If the container doesn't exist, that's fine, just move on.
pub async fn delete_container(container_name: &str) -> Result<()> {
    info!(
        "Removing any existing postgres container with name {}",
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
        "Removed any existing postgres container with name {}",
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

    // Check if the image is there. If not, print that we'll pull it.
    if docker.inspect_image(image_name).await.is_err() {
        eprintln!("Image {} not found, pulling it now", image_name);
    };

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
/// having the user use `docker logs` is the preferred approach. To help debug any
/// potential startup failures however, we also create files for the command to write to.
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

/// This shutdown step forcibly kills a container with the given name. If no container
/// is found we continue without error.
#[derive(Clone, Debug)]
pub struct KillContainerShutdownStep {
    container_name: &'static str,
}

impl KillContainerShutdownStep {
    pub fn new(container_name: &'static str) -> Self {
        Self { container_name }
    }
}

#[async_trait]
impl ShutdownStep for KillContainerShutdownStep {
    async fn run(self: Box<Self>) -> Result<()> {
        delete_container(self.container_name).await?;
        Ok(())
    }
}
