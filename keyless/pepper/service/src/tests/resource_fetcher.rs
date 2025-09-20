// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::PepperServiceError,
    external_resources::{
        groth16_vk::OnChainGroth16VerificationKey,
        keyless_config::OnChainKeylessConfiguration,
        resource_fetcher::{
            CachedResources, ENV_ONCHAIN_GROTH16_VK_URL, ENV_ONCHAIN_KEYLESS_CONFIG_URL,
            RESOURCE_FETCH_INTERVAL_SECS,
        },
    },
    request_handler::{GROTH16_VK_PATH, KEYLESS_CONFIG_PATH},
};
use aptos_time_service::TimeService;
use serial_test::serial;
use std::time::Duration;
use tokio::{task::JoinHandle, time::timeout};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};
// TODO: avoid using environment variables in code and tests!

// Maximum wait time for each test to complete
const MAX_TEST_WAIT_SECS: u64 = 10;

#[tokio::test(flavor = "multi_thread")]
#[serial] // Ensure tests using env vars run serially
async fn cached_resources_updates_after_interval() {
    // Create on-chain test resources
    let on_chain_keyless_configuration = OnChainKeylessConfiguration::new_for_testing();
    let on_chain_groth16_vk = OnChainGroth16VerificationKey::new_for_testing();

    // Create a mock server that returns the test resources
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(KEYLESS_CONFIG_PATH))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(on_chain_keyless_configuration.clone()),
        )
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path(GROTH16_VK_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(on_chain_groth16_vk.clone()))
        .mount(&mock_server)
        .await;

    // Verify that the cached resources are updated correctly
    verify_cached_resource_updates(
        on_chain_keyless_configuration,
        on_chain_groth16_vk,
        &mock_server,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial] // Ensure tests using env vars run serially
async fn cached_resources_updates_after_failures() {
    // Create on-chain test resources
    let on_chain_keyless_configuration = OnChainKeylessConfiguration::new_for_testing();
    let on_chain_groth16_vk = OnChainGroth16VerificationKey::new_for_testing();

    // Create a mock server with failing responses first, then successes
    let mock_server = MockServer::start().await;

    // The first two calls to the keyless endpoint will fail with 500, then succeed
    Mock::given(method("GET"))
        .and(path(KEYLESS_CONFIG_PATH))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path(KEYLESS_CONFIG_PATH))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(on_chain_keyless_configuration.clone()),
        )
        .mount(&mock_server)
        .await;

    // The first two calls to the Groth16 VK endpoint will fail with a missing response body, then succeed
    Mock::given(method("GET"))
        .and(path(GROTH16_VK_PATH))
        .respond_with(ResponseTemplate::new(200))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path(GROTH16_VK_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(on_chain_groth16_vk.clone()))
        .mount(&mock_server)
        .await;

    // Verify that the cached resources are updated correctly
    verify_cached_resource_updates(
        on_chain_keyless_configuration,
        on_chain_groth16_vk,
        &mock_server,
    )
    .await;
}

/// Advances the mock time service by the given number of seconds
async fn advance_time_secs(time_service: TimeService, seconds: u64) {
    let mock_time_service = time_service.into_mock();
    mock_time_service
        .advance_async(Duration::from_secs(seconds))
        .await;
}

/// Sets the environment variables for the external resource URLs
fn set_external_resource_env_variables(keyless_config_url: &String, groth16_vk_url: &String) {
    unsafe {
        std::env::set_var(ENV_ONCHAIN_KEYLESS_CONFIG_URL, &keyless_config_url);
        std::env::set_var(ENV_ONCHAIN_GROTH16_VK_URL, &groth16_vk_url);
    }
}

/// Removes the environment variables for the external resource URLs
fn remove_external_resource_env_variables() {
    unsafe {
        std::env::remove_var(ENV_ONCHAIN_KEYLESS_CONFIG_URL);
        std::env::remove_var(ENV_ONCHAIN_GROTH16_VK_URL);
    }
}

/// Verifies that the cached resources are eventually updated
/// correctly after the resource fetcher is started.
async fn verify_cached_resource_updates(
    on_chain_keyless_configuration: OnChainKeylessConfiguration,
    on_chain_groth16_vk: OnChainGroth16VerificationKey,
    mock_server: &MockServer,
) {
    // Set the environment variables to point to the mock server
    let keyless_config_url = format!("{}{}", &mock_server.uri(), KEYLESS_CONFIG_PATH);
    let groth16_vk_url = format!("{}{}", &mock_server.uri(), GROTH16_VK_PATH);
    set_external_resource_env_variables(&keyless_config_url, &groth16_vk_url);

    // Create the cached resources instance with a mock time service
    let time_service = TimeService::mock();
    let cached_resources = CachedResources::new(time_service.clone());

    // Start the cached resource fetcher
    cached_resources.start_resource_fetcher();

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
                advance_time_secs(time_service.clone(), RESOURCE_FETCH_INTERVAL_SECS + 1).await;

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

    // Clean up the environment variables
    remove_external_resource_env_variables();
}
