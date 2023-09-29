// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::traits::ShutdownStep;
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use reqwest::Url;
use std::{fs::create_dir_all, net::SocketAddr, path::Path, process::Stdio};
use tokio::process::Command;
use tracing::info;

pub fn socket_addr_to_url(socket_addr: &SocketAddr, scheme: &str) -> Result<Url> {
    let host = match socket_addr {
        SocketAddr::V4(v4) => format!("{}", v4.ip()),
        SocketAddr::V6(v6) => format!("[{}]", v6.ip()),
    };
    let full_url = format!("{}://{}:{}", scheme, host, socket_addr.port());
    Ok(Url::parse(&full_url)?)
}

pub async fn confirm_docker_available() -> Result<()> {
    let status = Command::new("docker")
        .arg("info")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .context("Failed to check if Docker is available")?;

    if !status.success() {
        bail!(
            "Docker is not available, confirm it is installed and running: {:?}",
            status
        );
    }

    Ok(())
}

/// Delete a container. If the container doesn't exist, that's fine, just move on.
pub async fn delete_container(container_name: &str) -> Result<()> {
    info!(
        "Removing any existing postgres container with name {}",
        container_name
    );

    let _ = Command::new("docker")
        .arg("rm")
        .arg("-f")
        .arg(container_name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .context(format!(
            "Failed to remove existing container with name {}",
            container_name
        ))?;

    info!(
        "Removed any existing postgres container with name {}",
        container_name
    );

    Ok(())
}

pub async fn pull_docker_image(image_name: &str) -> Result<()> {
    info!("Pulling docker image {}", image_name);
    let status = Command::new("docker")
        .arg("pull")
        .arg(image_name)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .context("Failed to pull postgres image")?;
    info!("Pulled docker image {}", image_name);

    if !status.success() {
        bail!("Failed to pull postgres image: {:?}", status);
    }

    Ok(())
}

/// This function creates a directory called `dir_name` under `test_dir` and writes a
/// file called README.md that tells the user where to go to see logs. We do this since
/// having the user use `docker logs` is the preferred approach. To help debug any
/// potential startup failures however, we also create files for the command to write to.
pub fn setup_docker_logging(
    test_dir: &Path,
    dir_name: &str,
    container_name: &str,
) -> Result<(Stdio, Stdio)> {
    // Create dir.
    let log_dir = test_dir.join(dir_name);
    create_dir_all(log_dir.as_path()).context(format!("Failed to create {}", log_dir.display()))?;

    // Write README.
    let data = format!(
        "To see logs for {} run the following command:\n\ndocker logs {}\n",
        dir_name, container_name
    );
    std::fs::write(log_dir.join("README.md"), data).context("Unable to write README file")?;

    // Create file for stdout.
    let path = log_dir.join("stdout.log");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(path.clone())
        .context(format!("Failed to create {}", path.display()))?;
    let stdout = Stdio::from(file);

    // Create file for stderr.
    let path = log_dir.join("stderr.log");
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(path.clone())
        .context(format!("Failed to create {}", path.display()))?;
    let stderr = Stdio::from(file);

    Ok((stdout, stderr))
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
