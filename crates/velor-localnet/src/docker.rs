// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use bollard::Docker;
#[cfg(unix)]
use bollard::API_DEFAULT_VERSION;
use tracing::{info, warn};
use version_compare::Version;

const ERROR_MESSAGE: &str = "Docker is not available, confirm it is installed and running. See https://velor.dev/guides/local-development-network#faq for assistance.";

/// This function returns a Docker client. Before returning, it confirms that it can
/// actually query the API and checks that the API version is sufficient. It first
/// tries to connect at the default socket location and if that fails, it tries to find
/// a socket in the user's home directory. On Windows NT it doesn't try that since
/// there no second location, there is just the one named pipe.
pub async fn get_docker() -> Result<Docker> {
    let docker = Docker::connect_with_local_defaults()
        .context(format!("{} (init_default)", ERROR_MESSAGE))
        .inspect_err(|e| eprintln!("{:#}", e))?;

    // We have to specify the type because the compiler can't figure out the error
    // in the case where the system is Unix.
    let out: Result<(Docker, bollard::system::Version), bollard::errors::Error> =
        match docker.version().await {
            Ok(version) => Ok((docker, version)),
            Err(err) => {
                warn!(
                    "Received this error trying to use default Docker socket location: {:#}",
                    err
                );
                // Look for the socket in ~/.docker/run
                // We don't have to do this if this issue gets addressed:
                // https://github.com/fussybeaver/bollard/issues/345
                #[cfg(unix)]
                {
                    let path = dirs::home_dir()
                        .context(format!("{} (home_dir)", ERROR_MESSAGE))?
                        .join(".docker")
                        .join("run")
                        .join("docker.sock");
                    info!("Looking for Docker socket at {}", path.display());
                    let path = path.to_str().context(format!("{} (path)", ERROR_MESSAGE))?;
                    let docker = Docker::connect_with_socket(path, 120, API_DEFAULT_VERSION)
                        .context(format!("{} (init_home)", ERROR_MESSAGE))?;
                    let version = docker
                        .version()
                        .await
                        .context(format!("{} (version_home)", ERROR_MESSAGE))?;
                    Ok((docker, version))
                }
                // Just return the original error.
                #[cfg(not(unix))]
                Err(err)
            },
        };
    let (docker, version) = out?;

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
                    info!("Docker version is sufficient: {}", current_api_version);
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

    Ok(docker)
}
