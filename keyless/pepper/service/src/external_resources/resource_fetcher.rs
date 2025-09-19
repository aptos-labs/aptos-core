// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    external_resources::{
        groth16_vk::OnChainGroth16VerificationKey, keyless_config::OnChainKeylessConfiguration,
    },
    metrics, utils,
};
use aptos_infallible::RwLock;
use aptos_logger::{info, prelude::SampleRate, sample, warn};
use serde::de::DeserializeOwned;
use std::{sync::Arc, time::Duration};
use tokio::time::Instant;

// Cached resource constants
const ENV_ONCHAIN_KEYLESS_CONFIG_URL: &str = "ONCHAIN_KEYLESS_CONFIG_URL";
const ENV_ONCHAIN_GROTH16_VK_URL: &str = "ONCHAIN_GROTH16_VK_URL";
const ONCHAIN_KEYLESS_CONFIG_RESOURCE_NAME: &str = "onchain_keyless_configuration";
const ONCHAIN_GROTH16_VK_RESOURCE_NAME: &str = "onchain_groth16_verification_key";
const RESOURCE_FETCH_INTERVAL_SECS: u64 = 10;
const RESOURCE_FETCH_LOG_INTERVAL_SECS: u64 = 60;

/// A struct that holds the cached resources and their refresh logic
#[derive(Clone, Debug, Default)]
pub struct CachedResources {
    on_chain_keyless_configuration: Arc<RwLock<Option<OnChainKeylessConfiguration>>>,
    groth16_vk: Arc<RwLock<Option<OnChainGroth16VerificationKey>>>,
}

impl CachedResources {
    /// Starts the refresh loops for the cached resources
    pub fn start_resource_fetcher(&self) {
        // Start the keyless config fetcher
        match utils::read_environment_variable(ENV_ONCHAIN_KEYLESS_CONFIG_URL) {
            Ok(url) => {
                start_external_resource_refresh_loop(
                    ONCHAIN_KEYLESS_CONFIG_RESOURCE_NAME.into(),
                    url,
                    self.on_chain_keyless_configuration.clone(),
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
) {
    info!(
        "Starting the cached resource refresh loop for {}!",
        resource_url
    );

    // Create the request client
    let client = utils::create_request_client();

    // Start the resource fetcher loop
    tokio::spawn(async move {
        loop {
            // Sleep until the next refresh
            let refresh_interval = Duration::from_secs(RESOURCE_FETCH_INTERVAL_SECS);
            tokio::time::sleep(refresh_interval).await;

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
                    sample!(
                        SampleRate::Duration(Duration::from_secs(RESOURCE_FETCH_LOG_INTERVAL_SECS)),
                        info!(
                            "Successfully fetched resource {} from {} in {:?}",
                            resource_name, resource_url, fetch_time
                        )
                    );

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
            }
        }
    });
}

/// Creates and starts the cached resource fetcher, and
/// returns a handle to the cached resources.
pub fn start_cached_resource_fetcher() -> CachedResources {
    // Create and start the fetcher
    let cached_resources = CachedResources::default();
    cached_resources.start_resource_fetcher();
    cached_resources
}
