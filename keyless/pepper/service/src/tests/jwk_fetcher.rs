// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::PepperServiceError,
    external_resources::{
        jwk_fetcher,
        jwk_fetcher::{JWKCache, KeyID, JWK_REFRESH_INTERVAL_SECS},
    },
};
use aptos_infallible::Mutex;
use aptos_time_service::TimeService;
use aptos_types::{jwks, jwks::rsa::RSA_JWK};
use serial_test::serial;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{task::JoinHandle, time::timeout};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

// Test issuer and JWK path constants for Apple
const ISSUER_APPLE: &str = "test_apple_issuer";
const JWK_PATH_APPLE: &str = "/auth/keys";

// Test issuer and JWK path constants for Google
const ISSUER_GOOGLE: &str = "test_google_issuer";
const JWK_PATH_GOOGLE: &str = "/oauth2/v3/certs";

// A test key ID for the JWKs
const TEST_KEY_ID: &str = "test_key_id";

// Maximum wait time for each test to complete
const MAX_TEST_WAIT_SECS: u64 = 10;

#[tokio::test(flavor = "multi_thread")]
#[serial] // Run serially to avoid interference
async fn jwk_cache_updates_after_interval() {
    // Create test JWK key sets
    let apple_jwk_key_set = create_test_jwk_key_set();
    let google_jwk_key_set = create_test_jwk_key_set();

    // Create a mock server that returns the key sets
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(JWK_PATH_APPLE))
        .respond_with(ResponseTemplate::new(200).set_body_json(apple_jwk_key_set.clone()))
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path(JWK_PATH_GOOGLE))
        .respond_with(ResponseTemplate::new(200).set_body_json(google_jwk_key_set.clone()))
        .mount(&mock_server)
        .await;

    // Verify that the JWK cache is updated correctly
    verify_jwk_cache_updates(apple_jwk_key_set, google_jwk_key_set, &mock_server).await;
}

#[tokio::test(flavor = "multi_thread")]
#[serial] // Run serially to avoid interference
async fn jwk_cache_updates_after_failures() {
    // Create test JWK key sets
    let apple_jwk_key_set = create_test_jwk_key_set();
    let google_jwk_key_set = create_test_jwk_key_set();

    // Create a mock server with failing responses first, then successes
    let mock_server = MockServer::start().await;

    // The first two calls to the Apple endpoint will fail with 500, then succeed
    Mock::given(method("GET"))
        .and(path(JWK_PATH_APPLE))
        .respond_with(ResponseTemplate::new(500))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path(JWK_PATH_APPLE))
        .respond_with(ResponseTemplate::new(200).set_body_json(apple_jwk_key_set.clone()))
        .mount(&mock_server)
        .await;

    // The first two calls to the Google endpoint will fail with a missing response body, then succeed
    Mock::given(method("GET"))
        .and(path(JWK_PATH_GOOGLE))
        .respond_with(ResponseTemplate::new(200))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path(JWK_PATH_GOOGLE))
        .respond_with(ResponseTemplate::new(200).set_body_json(google_jwk_key_set.clone()))
        .mount(&mock_server)
        .await;

    // Verify that the JWK cache is updated correctly
    verify_jwk_cache_updates(apple_jwk_key_set, google_jwk_key_set, &mock_server).await;
}

/// Advances the mock time service by the given number of seconds
async fn advance_time_secs(time_service: TimeService, seconds: u64) {
    let mock_time_service = time_service.into_mock();
    mock_time_service
        .advance_async(Duration::from_secs(seconds))
        .await;
}

/// Creates a JWK cache and starts the refresh loops for known issuers
fn create_and_start_jwk_cache(
    apple_jwk_url: String,
    google_jwk_url: String,
    time_service: TimeService,
) -> JWKCache {
    // Create the JWK cache
    let jwk_cache = Arc::new(Mutex::new(HashMap::new()));

    // Start the JWK refresh loops for known issuers
    jwk_fetcher::start_jwk_refresh_loop(
        ISSUER_APPLE.into(),
        apple_jwk_url,
        jwk_cache.clone(),
        time_service.clone(),
    );
    jwk_fetcher::start_jwk_refresh_loop(
        ISSUER_GOOGLE.into(),
        google_jwk_url,
        jwk_cache.clone(),
        time_service.clone(),
    );

    jwk_cache
}

/// Creates a test JWK key set with a single RSA key
fn create_test_jwk_key_set() -> HashMap<KeyID, Arc<RSA_JWK>> {
    let mut key_set = HashMap::new();
    key_set.insert(TEST_KEY_ID.into(), Arc::new(jwks::insecure_test_rsa_jwk()));
    key_set
}

/// Verifies that the cached resources are eventually updated
/// correctly after the resource fetcher is started.
async fn verify_jwk_cache_updates(
    apple_jwk_key_set: HashMap<KeyID, Arc<RSA_JWK>>,
    google_jwk_key_set: HashMap<KeyID, Arc<RSA_JWK>>,
    mock_server: &MockServer,
) {
    // Create the JWK URLs pointing to the mock server
    let apple_jwk_url = format!("{}{}", &mock_server.uri(), JWK_PATH_APPLE);
    let google_jwk_url = format!("{}{}", &mock_server.uri(), JWK_PATH_GOOGLE);

    // Create the JWK cache and start the refresh loops
    let time_service = TimeService::mock();
    let jwk_cache = create_and_start_jwk_cache(apple_jwk_url, google_jwk_url, time_service.clone());

    // Verify that initially the cache is empty
    assert!(jwk_cache.lock().get(ISSUER_APPLE).is_none());
    assert!(jwk_cache.lock().get(ISSUER_GOOGLE).is_none());

    // Spawn a task that advances time and verifies the cache is eventually updated
    let cache_verification_task: JoinHandle<Result<(), PepperServiceError>> = tokio::spawn({
        let time_service = time_service.clone();
        let jwk_cache = jwk_cache.clone();

        async move {
            loop {
                // Advance time by the refresh interval
                advance_time_secs(time_service.clone(), JWK_REFRESH_INTERVAL_SECS + 1).await;

                // Grab the cached key sets
                let cached_apple_jwk_set = jwk_cache.lock().get(ISSUER_APPLE).cloned();
                let cached_google_jwk_set = jwk_cache.lock().get(ISSUER_GOOGLE).cloned();

                // Check if the cache has been updated correctly
                match (cached_apple_jwk_set, cached_google_jwk_set) {
                    (Some(cached_apple_jwk_set), Some(cached_google_jwk_set)) => {
                        assert_eq!(cached_apple_jwk_set, apple_jwk_key_set);
                        assert_eq!(cached_google_jwk_set, google_jwk_key_set);
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

    // Verify that the JWK cache is eventually updated
    if let Err(error) = timeout(
        Duration::from_secs(MAX_TEST_WAIT_SECS),
        cache_verification_task,
    )
    .await
    {
        panic!("Failed waiting for JWK cache to be updated: {}", error);
    }
}
