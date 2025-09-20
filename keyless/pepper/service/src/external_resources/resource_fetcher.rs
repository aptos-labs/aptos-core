// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    external_resources::{
        groth16_vk::OnChainGroth16VerificationKey, keyless_config::OnChainKeylessConfiguration,
    },
    metrics, utils,
};
use aptos_infallible::RwLock;
use aptos_logger::{info, warn};
use aptos_time_service::{TimeService, TimeServiceTrait};
use serde::de::DeserializeOwned;
use std::{sync::Arc, time::Duration};
use tokio::time::Instant;

// Environment variable names for the resource URLs
pub const ENV_ONCHAIN_KEYLESS_CONFIG_URL: &str = "ONCHAIN_KEYLESS_CONFIG_URL";
pub const ENV_ONCHAIN_GROTH16_VK_URL: &str = "ONCHAIN_GROTH16_VK_URL";

// Resource names for metrics and logging
const ONCHAIN_KEYLESS_CONFIG_RESOURCE_NAME: &str = "onchain_keyless_configuration";
const ONCHAIN_GROTH16_VK_RESOURCE_NAME: &str = "onchain_groth16_verification_key";

// The interval (in seconds) at which to refresh the resources
pub const RESOURCE_FETCH_INTERVAL_SECS: u64 = 10;

// The frequency at which to log the resource fetch status (per loop iteration)
const RESOURCE_REFRESH_LOOP_LOG_FREQUENCY: u64 = 6; // e.g., 6 * 10s (per loop) = 60s per log

/// A struct that holds the cached resources and their refresh logic
#[derive(Clone, Debug)]
pub struct CachedResources {
    on_chain_keyless_configuration: Arc<RwLock<Option<OnChainKeylessConfiguration>>>,
    groth16_vk: Arc<RwLock<Option<OnChainGroth16VerificationKey>>>,
    time_service: TimeService,
}

impl CachedResources {
    pub fn new(time_service: TimeService) -> Self {
        Self {
            on_chain_keyless_configuration: Arc::new(RwLock::new(None)),
            groth16_vk: Arc::new(RwLock::new(None)),
            time_service,
        }
    }

    /// Creates a new CachedResources instance with a mock time service (for testing)
    #[cfg(test)]
    pub fn new_for_testing() -> Self {
        Self::new(TimeService::mock())
    }

    /// Starts the refresh loops for the cached resources
    pub fn start_resource_fetcher(&self) {
        // Start the keyless config fetcher
        match utils::read_environment_variable(ENV_ONCHAIN_KEYLESS_CONFIG_URL) {
            Ok(url) => {
                start_external_resource_refresh_loop(
                    ONCHAIN_KEYLESS_CONFIG_RESOURCE_NAME.into(),
                    url,
                    self.on_chain_keyless_configuration.clone(),
                    self.time_service.clone(),
                );
            },
            Err(error) => {
                warn!(
                    "{} is not set, skipping on-chain keyless config fetcher! Error: {}",
                    ENV_ONCHAIN_KEYLESS_CONFIG_URL, error
                );
            },
        }

        // Start the Groth16 VK fetcher
        match utils::read_environment_variable(ENV_ONCHAIN_GROTH16_VK_URL) {
            Ok(url) => {
                start_external_resource_refresh_loop(
                    ONCHAIN_GROTH16_VK_RESOURCE_NAME.into(),
                    url,
                    self.groth16_vk.clone(),
                    self.time_service.clone(),
                );
            },
            Err(error) => {
                warn!(
                    "{} is not set, skipping on-chain Groth16 VK fetcher! Error: {}",
                    ENV_ONCHAIN_GROTH16_VK_URL, error
                );
            },
        }
    }

    /// Reads the cached on-chain Groth16 verification key
    pub fn read_on_chain_groth16_vk(&self) -> Option<OnChainGroth16VerificationKey> {
        self.groth16_vk.read().as_ref().cloned()
    }

    /// Reads the cached on-chain keyless configuration
    pub fn read_on_chain_keyless_configuration(&self) -> Option<OnChainKeylessConfiguration> {
        self.on_chain_keyless_configuration.read().as_ref().cloned()
    }

    /// Sets the cached on-chain Groth16 verification key (for testing purposes)
    #[cfg(test)]
    pub fn set_on_chain_groth16_vk(&self, on_chain_groth16_vk: OnChainGroth16VerificationKey) {
        let mut cache = self.groth16_vk.write();
        *cache = Some(on_chain_groth16_vk);
    }

    /// Sets the cached on-chain keyless configuration (for testing purposes)
    #[cfg(test)]
    pub fn set_on_chain_keyless_configuration(
        &self,
        on_chain_keyless_configuration: OnChainKeylessConfiguration,
    ) {
        let mut cache = self.on_chain_keyless_configuration.write();
        *cache = Some(on_chain_keyless_configuration);
    }
}

/// Starts a background task that periodically fetches and caches the resource from the given URL
fn start_external_resource_refresh_loop<T: DeserializeOwned + Send + Sync + 'static>(
    resource_name: String,
    resource_url: String,
    local_cache: Arc<RwLock<Option<T>>>,
    time_service: TimeService,
) {
    info!(
        "Starting the cached resource refresh loop for {}!",
        resource_url
    );

    // Create the request client
    let client = utils::create_request_client();

    // Start the resource fetcher loop
    tokio::spawn(async move {
        let mut loop_iteration_counter: u64 = 0;

        loop {
            // Sleep until the next refresh
            let refresh_interval = Duration::from_secs(RESOURCE_FETCH_INTERVAL_SECS);
            time_service.sleep(refresh_interval).await;

            // Increment the loop iteration counter
            loop_iteration_counter = loop_iteration_counter.wrapping_add(1);

            // Fetch the resource from the URL
            let time_now = Instant::now();
            let fetch_result = client.get(resource_url.clone()).send().await;
            let fetch_time = time_now.elapsed();

            // Update the fetch metrics
            metrics::update_external_resource_fetch_metrics(
                &resource_name,
                fetch_result.is_ok(),
                fetch_time,
            );

            // Process the fetch result
            match fetch_result {
                Ok(response) => {
                    // Log the successful fetch
                    if loop_iteration_counter % RESOURCE_REFRESH_LOOP_LOG_FREQUENCY == 0 {
                        info!(
                            "Successfully fetched resource {} from {} in {:?}",
                            resource_name, resource_url, fetch_time
                        )
                    }

                    // Parse the response into the expected resource
                    let resource = match response.json::<T>().await {
                        Ok(resource) => resource,
                        Err(error) => {
                            warn!(
                                "Failed to parse resource from {}! Error: {}",
                                resource_url, error
                            );
                            continue; // Retry in the next loop
                        },
                    };

                    // Update the cache
                    let mut cache = local_cache.write();
                    *cache = Some(resource);
                },
                Err(error) => {
                    warn!(
                        "Failed to fetch resource from {} in {:?}! Error: {}",
                        resource_url, fetch_time, error
                    );
                    continue; // Retry in the next loop
                },
            };
        }
    });
}

/// Creates and starts the cached resource fetcher, and
/// returns a handle to the cached resources.
pub fn start_cached_resource_fetcher() -> CachedResources {
    // Create and start the fetcher
    let cached_resources = CachedResources::new(TimeService::real());
    cached_resources.start_resource_fetcher();
    cached_resources
}
