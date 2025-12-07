// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    error::PepperServiceError,
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

// TODO: at some point, we should try to merge the JWK and resource fetcher code

// Resource names for metrics and logging
const ON_CHAIN_KEYLESS_CONFIG_RESOURCE_NAME: &str = "on_chain_keyless_configuration";
const ON_CHAIN_GROTH16_VK_RESOURCE_NAME: &str = "on_chain_groth16_verification_key";

// The interval (in seconds) at which to refresh the resources
pub const RESOURCE_FETCH_INTERVAL_SECS: u64 = 10;

// The frequency at which to log the resource fetch status (per loop iteration)
const RESOURCE_REFRESH_LOOP_LOG_FREQUENCY: u64 = 6; // e.g., 6 * 10s (per loop) = 60s per log

/// A common interface for fetching external resources (this is especially useful for logging and testing)
#[async_trait::async_trait]
pub trait ExternalResourceInterface<T: DeserializeOwned + Send + Sync + 'static> {
    /// Returns the name of the resource
    fn resource_name(&self) -> String;

    /// Returns the URL of the resource
    fn resource_url(&self) -> String;

    /// Fetches the resource from the URL and parses it into the expected type
    async fn fetch_resource(&self) -> Result<T, PepperServiceError>;
}

/// An external resource (e.g., on-chain keyless config or Groth16 VK)
struct ExternalResource {
    name: String,
    url: String,
}

impl ExternalResource {
    pub fn new(name: String, url: String) -> ExternalResource {
        ExternalResource { name, url }
    }
}

#[async_trait::async_trait]
impl<T: DeserializeOwned + Send + Sync + 'static> ExternalResourceInterface<T>
    for ExternalResource
{
    fn resource_name(&self) -> String {
        self.name.clone()
    }

    fn resource_url(&self) -> String {
        self.url.clone()
    }

    async fn fetch_resource(&self) -> Result<T, PepperServiceError> {
        // Fetch the resource from the URL
        let url = self.url.clone();
        let client = utils::create_request_client();
        let fetch_result = client.get(url.clone()).send().await;

        // Parse the response into the expected resource type
        let resource_name = self.name.clone();
        match fetch_result {
            Ok(response) => match response.json::<T>().await {
                Ok(resource) => Ok(resource),
                Err(error) => Err(PepperServiceError::InternalError(format!(
                    "Failed to parse resource: {}, from {}! Error: {}",
                    resource_name, url, error
                ))),
            },
            Err(error) => Err(PepperServiceError::InternalError(format!(
                "Failed to fetch resource: {}, from {}! Error: {}",
                resource_name, url, error
            ))),
        }
    }
}

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

    /// Starts the refresh loops for the cached resources at the given URLs.
    /// If a URL is None, the corresponding fetcher is not started.
    pub fn start_resource_fetcher(
        &self,
        on_chain_groth16_vk_url: Option<String>,
        on_chain_keyless_config_url: Option<String>,
    ) {
        // Start the Groth16 VK fetcher
        match on_chain_groth16_vk_url {
            Some(url) => {
                let external_resource = Arc::new(ExternalResource::new(
                    ON_CHAIN_GROTH16_VK_RESOURCE_NAME.into(),
                    url.clone(),
                ));
                start_external_resource_refresh_loop(
                    external_resource,
                    self.groth16_vk.clone(),
                    self.time_service.clone(),
                );
            },
            None => {
                warn!("The on-chain Groth16 VK URL is not set, skipping on-chain Groth16 VK fetching!");
            },
        }

        // Start the keyless config fetcher
        match on_chain_keyless_config_url {
            Some(url) => {
                let external_resource = Arc::new(ExternalResource::new(
                    ON_CHAIN_KEYLESS_CONFIG_RESOURCE_NAME.into(),
                    url.clone(),
                ));
                start_external_resource_refresh_loop(
                    external_resource,
                    self.on_chain_keyless_configuration.clone(),
                    self.time_service.clone(),
                );
            },
            None => {
                warn!("The on-chain keyless config URL is not set, skipping on-chain keyless config fetching!");
            },
        }
    }

    /// Returns the Groth16 verification key cache entry (for testing purposes)
    #[cfg(test)]
    pub fn get_groth16_vk_cache_entry(&self) -> Arc<RwLock<Option<OnChainGroth16VerificationKey>>> {
        self.groth16_vk.clone()
    }

    /// Returns the keyless configuration cache entry (for testing purposes)
    #[cfg(test)]
    pub fn get_keyless_configuration_cache_entry(
        &self,
    ) -> Arc<RwLock<Option<OnChainKeylessConfiguration>>> {
        self.on_chain_keyless_configuration.clone()
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
pub fn start_external_resource_refresh_loop<T: DeserializeOwned + Send + Sync + 'static>(
    external_resource: Arc<dyn ExternalResourceInterface<T> + Send + Sync>,
    local_cache: Arc<RwLock<Option<T>>>,
    time_service: TimeService,
) {
    // Log the start of the task for the resource fetcher
    let resource_name = external_resource.resource_name();
    let resource_url = external_resource.resource_url();
    info!(
        "Starting the cached resource refresh loop for {} at {}!",
        resource_name, resource_url
    );

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
            let fetch_result: Result<T, PepperServiceError> =
                external_resource.fetch_resource().await;
            let fetch_time = time_now.elapsed();

            // Update the fetch metrics
            metrics::update_external_resource_fetch_metrics(
                &resource_name,
                fetch_result.is_ok(),
                fetch_time,
            );

            // Process the fetch result
            match fetch_result {
                Ok(resource) => {
                    // Log the successful fetch
                    if loop_iteration_counter % RESOURCE_REFRESH_LOOP_LOG_FREQUENCY == 0 {
                        info!(
                            "Successfully fetched resource {} from {} in {:?}",
                            resource_name, resource_url, fetch_time
                        )
                    }

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
pub fn start_cached_resource_fetcher(
    on_chain_groth16_vk_url: Option<String>,
    on_chain_keyless_config_url: Option<String>,
) -> CachedResources {
    let cached_resources = CachedResources::new(TimeService::real());
    cached_resources.start_resource_fetcher(on_chain_groth16_vk_url, on_chain_keyless_config_url);
    cached_resources
}
