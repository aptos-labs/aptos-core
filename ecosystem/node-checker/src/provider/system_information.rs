// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    cache::OutputCache,
    traits::{Provider, ProviderError},
    CommonProviderConfig,
};
use crate::checker::CheckResult;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SystemInformationProviderConfig {
    #[serde(default, flatten)]
    pub common: CommonProviderConfig,
}

#[derive(Clone, Debug)]
pub struct SystemInformationProvider {
    pub config: SystemInformationProviderConfig,

    client: Arc<reqwest::Client>,

    /// This has both the path and the port already rolled into it.
    metrics_url: Url,

    // This output cache helps prevent the Provider from overfetching the data within
    // a short window of time. Downstream Checkers should be aware of this behaviour.
    output_cache: Arc<OutputCache<SystemInformation>>,
}

impl SystemInformationProvider {
    pub fn new(
        config: SystemInformationProviderConfig,
        client: Arc<reqwest::Client>,
        mut url: Url,
        metrics_port: u16,
    ) -> Self {
        url.set_path("system_information");
        url.set_port(Some(metrics_port)).unwrap();
        let output_cache = Arc::new(OutputCache::new(Duration::from_millis(
            config.common.cache_ttl_ms,
        )));
        Self {
            config,
            client,
            metrics_url: url,
            output_cache,
        }
    }

    pub async fn get_data(&self) -> Result<SystemInformation, ProviderError> {
        let response = self
            .client
            .get(self.metrics_url.clone())
            .send()
            .await
            .with_context(|| format!("Failed to get data from {}", self.metrics_url))
            .map_err(|e| ProviderError::RetryableEndpointError("/system_information", e))?;
        let text = response
            .text()
            .await
            .with_context(|| {
                format!(
                    "Failed to process response body from {} as text",
                    self.metrics_url
                )
            })
            .map_err(|e| ProviderError::ParseError(anyhow!(e)))?;
        let data: HashMap<String, String> = serde_json::from_str(&text)
            .with_context(|| {
                format!(
                    "Failed to process response body from {} as valid JSON with string key/values",
                    self.metrics_url
                )
            })
            .map_err(|e| ProviderError::ParseError(anyhow!(e)))?;
        Ok(SystemInformation(data))
    }
}

#[async_trait]
impl Provider for SystemInformationProvider {
    type Output = SystemInformation;

    async fn provide(&self) -> Result<Self::Output, ProviderError> {
        self.output_cache.get(self.get_data()).await
    }

    fn explanation() -> &'static str {
        "The metrics port was not included in the request."
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SystemInformation(pub HashMap<String, String>);

/// This is a convenience function that returns the value if it was
/// found, or a CheckResult if not.
pub fn get_value<F>(
    data: &SystemInformation,
    metric_key: &str,
    evaluation_on_missing_fn: F,
) -> GetValueResult
where
    F: FnOnce() -> CheckResult,
{
    let metric_value = data.0.get(metric_key);
    match metric_value {
        Some(v) => GetValueResult::Present(v.to_string()),
        None => GetValueResult::Missing(evaluation_on_missing_fn()),
    }
}

pub enum GetValueResult {
    Present(String),
    Missing(CheckResult),
}
