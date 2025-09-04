// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::Runner;
use crate::{
    checker::{CheckResult, Checker, CheckerError},
    configuration::NodeAddress,
    provider::{
        api_index::ApiIndexProvider, metrics::MetricsProvider, noise::NoiseProvider,
        system_information::SystemInformationProvider, ProviderCollection, ProviderConfigs,
    },
    CheckSummary,
};
use anyhow::Result;
use velor_logger::{error, info, warn};
use async_trait::async_trait;
use futures::future::try_join_all;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SyncRunnerConfig {
    /// If > 0, Checkers that failed with a retryable error will be retried
    /// this many times with the configured delay.
    #[serde(default)]
    pub num_retries: u8,

    /// If num_retries > 0, this is the delay in seconds between retries.
    #[serde(default = "SyncRunnerConfig::default_retry_delay_secs")]
    pub retry_delay_secs: u16,
}

impl SyncRunnerConfig {
    fn default_retry_delay_secs() -> u16 {
        2
    }
}

impl Default for SyncRunnerConfig {
    fn default() -> Self {
        Self {
            num_retries: u8::default(),
            retry_delay_secs: Self::default_retry_delay_secs(),
        }
    }
}

#[derive(Debug)]
pub struct SyncRunner {
    /// Any arguments for the runner itself can be configured here.
    config: SyncRunnerConfig,

    /// At startup we may have been able to build some Providers already based
    /// on the information in the BaselineConfiguration. If so, those Providers
    /// will be in this ProviderCollection.
    provider_collection: ProviderCollection,

    /// This collection of configs can be used to build Providers at request time.
    provider_configs: ProviderConfigs,

    /// All the Checkers we built based on the BaselineConfiguration.
    checkers: Vec<Box<dyn Checker>>,
}

impl SyncRunner {
    pub fn new(
        config: SyncRunnerConfig,
        provider_collection: ProviderCollection,
        provider_configs: ProviderConfigs,
        checkers: Vec<Box<dyn Checker>>,
    ) -> Self {
        Self {
            config,
            provider_collection,
            provider_configs,
            checkers,
        }
    }
}

/// SyncRunner doesn't imply synchronous execution, but rather that it
/// is synchronous from the user's perspective, vs some kind of continuous
/// streaming back of (partial) results.
#[async_trait]
impl Runner for SyncRunner {
    async fn run(&self, target_node_address: &NodeAddress) -> Result<CheckSummary> {
        let now = std::time::Instant::now();
        info!(
            target_node_url = target_node_address.url,
            event = "check_starting"
        );

        // Here we build a ProviderCollection and try to build every Provider
        // we can based on the request. We clone the ProviderCollection from
        // the runner itself to start with, since it might already have some
        // prebuilt Providers in it (for the baseline node). Cloning this
        // ProviderCollection the nice property that the Providers within are
        // wrapped in Arcs, so we're still using the same Provider instances
        // between requests, allowing us to do some smart memoization.
        let mut provider_collection = self.provider_collection.clone();

        // Build the MetricsProvider for the target node.
        if let Ok(metrics_client) = target_node_address.get_metrics_client(Duration::from_secs(4)) {
            let metrics_client = Arc::new(metrics_client);
            provider_collection.target_metrics_provider = Some(MetricsProvider::new(
                self.provider_configs.metrics.clone(),
                metrics_client.clone(),
                target_node_address.url.clone(),
                target_node_address.get_metrics_port().unwrap(),
            ));
            provider_collection.target_system_information_provider =
                Some(SystemInformationProvider::new(
                    self.provider_configs.system_information.clone(),
                    metrics_client,
                    target_node_address.url.clone(),
                    target_node_address.get_metrics_port().unwrap(),
                ));
        }

        // Build the ApiIndexProvider for the target node.
        if let Ok(api_client) = target_node_address.get_api_client(Duration::from_secs(4)) {
            let api_index_provider = Arc::new(ApiIndexProvider::new(
                self.provider_configs.api_index.clone(),
                api_client,
            ));
            provider_collection.target_api_index_provider = Some(api_index_provider.clone());

            // From here, since we have an API provider, we can try to make a noise provider.
            if let (Some(_), Some(_)) = (
                target_node_address.get_noise_port(),
                target_node_address.get_public_key(),
            ) {
                // If the noise port and public key were provided but we can't parse
                // them as a network address, just fail early.
                let noise_address = match target_node_address.as_noise_network_address() {
                    Ok(noise_address) => noise_address,
                    Err(err) => {
                        return Ok(CheckSummary::from(vec![CheckResult::new(
                            "RequestHandler".to_string(),
                            "Invalid public key".to_string(),
                            0,
                            format!("Failed to build noise address: {:#}", err),
                        )]));
                    },
                };
                provider_collection.target_noise_provider = Some(NoiseProvider::new(
                    self.provider_configs.noise.clone(),
                    noise_address,
                    api_index_provider,
                ));
            }
        }

        // Call each of the Checkers without awaiting them yet.
        let mut futures = Vec::new();
        for checker in &self.checkers {
            futures.push(self.call_check(checker, &provider_collection));
        }

        // Run all the Checkers concurrently and collect their results.
        let check_results: Vec<CheckResult> =
            try_join_all(futures).await?.into_iter().flatten().collect();

        let check_summary = CheckSummary::from(check_results);

        info!(
            target_node_url = target_node_address.url,
            event = "check_successful",
            num_check_results = check_summary.check_results.len(),
            elapsed_ms = now.elapsed().as_millis() as u64,
            overall_score = check_summary.summary_score,
        );
        Ok(check_summary)
    }
}

impl SyncRunner {
    /// This function handles calling a checker multiple times if it failed with
    /// a retryable policy and the Runner is configured with a retry policy.
    #[allow(clippy::borrowed_box)]
    async fn call_check(
        &self,
        checker: &Box<dyn Checker>,
        provider_collection: &ProviderCollection,
    ) -> Result<Vec<CheckResult>, CheckerError> {
        let mut num_attempts = 0;
        let check_result = loop {
            match checker.check(provider_collection).await {
                Ok(check_result) => break check_result,
                Err(err) => {
                    if num_attempts < self.config.num_retries {
                        num_attempts += 1;
                        warn!(
                            "Checker failed with a retryable error: {:#}. Retrying in {} seconds.",
                            err, self.config.retry_delay_secs
                        );
                        tokio::time::sleep(Duration::from_secs(
                            self.config.retry_delay_secs.into(),
                        ))
                        .await;
                    } else {
                        error!(
                            "Checker failed with a retryable error too many times ({}): {:#}.",
                            self.config.num_retries, err
                        );
                        return Err(err);
                    }
                },
            }
        };
        Ok(check_result)
    }
}
