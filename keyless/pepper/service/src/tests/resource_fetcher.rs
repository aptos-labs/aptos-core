// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use crate::{
    error::PepperServiceError,
    external_resources::{
        groth16_vk::OnChainGroth16VerificationKey,
        keyless_config::OnChainKeylessConfiguration,
        resource_fetcher,
        resource_fetcher::{
            CachedResources, ExternalResourceInterface, RESOURCE_FETCH_INTERVAL_SECS,
        },
    },
    tests::utils,
};
use aptos_infallible::Mutex;
use aptos_time_service::TimeService;
use serde::de::DeserializeOwned;
use std::{sync::Arc, time::Duration};
use tokio::{task::JoinHandle, time::timeout};

// Maximum wait time (secs) for each test to complete
const MAX_TEST_WAIT_SECS: u64 = 10;

/// A mock external resource (for testing resource fetching and caching)
struct MockExternalResource<T> {
    resource_name: String,
    resource: T,
    num_fetch_failures: Arc<Mutex<u64>>, // The number of fetch failures to simulate
}

impl<T: DeserializeOwned + Send + Sync + 'static> MockExternalResource<T> {
    pub fn new(resource_name: String, resource: T, num_fetch_failures: u64) -> Self {
        Self {
            resource_name,
            resource,
            num_fetch_failures: Arc::new(Mutex::new(num_fetch_failures)),
        }
    }
}

#[async_trait::async_trait]
impl<T: DeserializeOwned + Clone + Send + Sync + 'static> ExternalResourceInterface<T>
    for MockExternalResource<T>
{
    fn resource_name(&self) -> String {
        self.resource_name.clone()
    }

    fn resource_url(&self) -> String {
        "".into() // The URL is not used in the mock implementation
    }

    async fn fetch_resource(&self) -> Result<T, PepperServiceError> {
        // If there are failures to simulate, decrement the counter and
        // return an error, otherwise, return the resource.
        let mut num_fetch_failures = self.num_fetch_failures.lock();
        if *num_fetch_failures > 0 {
            *num_fetch_failures -= 1;
            Err(PepperServiceError::InternalError(format!(
                "Simulated fetch failure for resource: {}! Failures remaining: {}",
                self.resource_name, *num_fetch_failures
            )))
        } else {
            Ok(self.resource.clone())
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn cached_resources_updates_after_interval() {
    // Create test external resources
    let on_chain_keyless_configuration = OnChainKeylessConfiguration::new_for_testing();
    let on_chain_groth16_vk = OnChainGroth16VerificationKey::new_for_testing();

    // Create mock external resources
    let keyless_resource = Arc::new(MockExternalResource::new(
        "OnChainKeylessConfiguration".into(),
        on_chain_keyless_configuration.clone(),
        0,
    ));
    let groth16_vk_resource = Arc::new(MockExternalResource::new(
        "OnChainGroth16VerificationKey".into(),
        on_chain_groth16_vk.clone(),
        0,
    ));

    // Verify that the cached resources are updated correctly
    verify_cached_resource_updates(
        on_chain_keyless_configuration,
        on_chain_groth16_vk,
        keyless_resource,
        groth16_vk_resource,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn cached_resources_updates_after_failures() {
    // Create test external resources
    let on_chain_keyless_configuration = OnChainKeylessConfiguration::new_for_testing();
    let on_chain_groth16_vk = OnChainGroth16VerificationKey::new_for_testing();

    // Create mock external resources that fail a few times before succeeding
    let num_fetch_failures = 4;
    let keyless_resource = Arc::new(MockExternalResource::new(
        "OnChainKeylessConfiguration".into(),
        on_chain_keyless_configuration.clone(),
        num_fetch_failures,
    ));
    let groth16_vk_resource = Arc::new(MockExternalResource::new(
        "OnChainGroth16VerificationKey".into(),
        on_chain_groth16_vk.clone(),
        num_fetch_failures,
    ));

    // Verify that the cached resources are updated correctly
    verify_cached_resource_updates(
        on_chain_keyless_configuration,
        on_chain_groth16_vk,
        keyless_resource,
        groth16_vk_resource,
    )
    .await;
}

/// Verifies that the cached resources are eventually updated
/// correctly after the resource fetcher is started.
async fn verify_cached_resource_updates(
    on_chain_keyless_configuration: OnChainKeylessConfiguration,
    on_chain_groth16_vk: OnChainGroth16VerificationKey,
    keyless_resource: Arc<dyn ExternalResourceInterface<OnChainKeylessConfiguration> + Send + Sync>,
    groth16_vk_resource: Arc<
        dyn ExternalResourceInterface<OnChainGroth16VerificationKey> + Send + Sync,
    >,
) {
    // Create the cached resources instance with a mock time service
    let time_service = TimeService::mock();
    let cached_resources = CachedResources::new(time_service.clone());

    // Start the external resource fetchers
    resource_fetcher::start_external_resource_refresh_loop(
        keyless_resource,
        cached_resources.get_keyless_configuration_cache_entry(),
        time_service.clone(),
    );
    resource_fetcher::start_external_resource_refresh_loop(
        groth16_vk_resource,
        cached_resources.get_groth16_vk_cache_entry(),
        time_service.clone(),
    );

    // Verify that initially the cache is empty
    assert!(cached_resources
        .read_on_chain_keyless_configuration()
        .is_none());
    assert!(cached_resources.read_on_chain_groth16_vk().is_none());

    // Spawn a task that advances time and verifies the cache is eventually updated
    let cache_verification_task: JoinHandle<Result<(), PepperServiceError>> = tokio::spawn({
        let time_service = time_service.clone();
        let cached_resources = cached_resources.clone();

        async move {
            loop {
                // Advance time by the fetch interval
                utils::advance_time_secs(time_service.clone(), RESOURCE_FETCH_INTERVAL_SECS + 1)
                    .await;

                // Grab the cached resources
                let fetched_keyless_configuration =
                    cached_resources.read_on_chain_keyless_configuration();
                let fetched_groth16_vk = cached_resources.read_on_chain_groth16_vk();

                // Check if the cache has been updated correctly
                match (fetched_keyless_configuration, fetched_groth16_vk) {
                    (Some(fetched_keyless_configuration), Some(fetched_groth16_vk)) => {
                        assert_eq!(
                            fetched_keyless_configuration,
                            on_chain_keyless_configuration
                        );
                        assert_eq!(fetched_groth16_vk, on_chain_groth16_vk);
                        return Ok(());
                    },
                    _ => {
                        // Yield to allow other tasks to run
                        tokio::task::yield_now().await;
                    },
                }
            }
        }
    });

    // Verify that the cached resources are eventually updated
    if let Err(error) = timeout(
        Duration::from_secs(MAX_TEST_WAIT_SECS),
        cache_verification_task,
    )
    .await
    {
        panic!(
            "Failed waiting for cached resources to be updated: {}",
            error
        );
    }
}
