// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{anyhow, Context, Result};
use reqwest::Url;
use serde::Serialize;
use std::time::Duration;
use tokio::time::Instant;

const MAX_WAIT_S: u64 = 120;
const WAIT_INTERVAL_MS: u64 = 200;

/// This provides a single place to define a variety of different healthchecks. In
/// cases where the name of the service being checked isn't obvious, the enum will take
/// a string arg that names it.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub enum HealthChecker {
    /// Check that a HTTP API is up. The second param is the name of the HTTP service.
    Http(Url, String),
    /// Check that the node API is up. This is just a specific case of Http for extra
    /// guarantees around liveliness.
    NodeApi(Url),
}

impl HealthChecker {
    pub async fn check(&self) -> Result<()> {
        match self {
            HealthChecker::Http(url, _) => {
                reqwest::get(Url::clone(url))
                    .await
                    .with_context(|| format!("Failed to GET {}", url))?;
                Ok(())
            },
            HealthChecker::NodeApi(url) => {
                aptos_rest_client::Client::new(Url::clone(url))
                    .get_index()
                    .await?;
                Ok(())
            },
        }
    }

    /// Wait up to MAX_WAIT_S seconds for a service to start up.
    pub async fn wait(
        &self,
        // The service, if any, waiting for this service to start up.
        waiting_service: Option<&str>,
    ) -> Result<()> {
        let prefix = self.to_string();
        wait_for_startup(|| self.check(), match waiting_service {
            Some(waiting_service) => {
                format!(
                    "{} at {} did not start up before {}",
                    prefix,
                    self.address_str(),
                    waiting_service,
                )
            },
            None => format!("{} at {} did not start up", prefix, self.address_str()),
        })
        .await
    }

    /// This is only ever used for display purposes. If possible, this should be the
    /// endpoint of the service that this HealthChecker is checking.
    pub fn address_str(&self) -> &str {
        match self {
            HealthChecker::Http(url, _) => url.as_str(),
            HealthChecker::NodeApi(url) => url.as_str(),
        }
    }

    /// Given a port, make an instance of HealthChecker::Http targeting 127.0.0.1.
    pub fn http_checker_from_port(port: u16, name: String) -> Self {
        Self::Http(
            Url::parse(&format!("http://127.0.0.1:{}", port,)).unwrap(),
            name,
        )
    }
}

impl std::fmt::Display for HealthChecker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthChecker::Http(_, name) => write!(f, "{}", name),
            HealthChecker::NodeApi(_) => write!(f, "Node API"),
        }
    }
}

async fn wait_for_startup<F, Fut>(check_fn: F, error_message: String) -> Result<()>
where
    F: Fn() -> Fut,
    Fut: futures::Future<Output = Result<()>>,
{
    let max_wait = Duration::from_secs(MAX_WAIT_S);
    let wait_interval = Duration::from_millis(WAIT_INTERVAL_MS);

    let start = Instant::now();
    let mut started_successfully = false;

    let mut last_error_message = None;
    while start.elapsed() < max_wait {
        match check_fn().await {
            Ok(_) => {
                started_successfully = true;
                break;
            },
            Err(err) => {
                last_error_message = Some(format!("{:#}", err));
            },
        }
        tokio::time::sleep(wait_interval).await
    }

    if !started_successfully {
        let error_message = match last_error_message {
            Some(last_error_message) => format!("{}: {}", error_message, last_error_message),
            None => error_message,
        };
        return Err(anyhow!(error_message));
    }

    Ok(())
}
