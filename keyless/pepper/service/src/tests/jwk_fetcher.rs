// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    error::PepperServiceError,
    external_resources::{
        jwk_fetcher,
        jwk_fetcher::{JWKCache, JWKIssuerInterface, KeyID, JWK_REFRESH_INTERVAL_SECS},
    },
    tests::utils,
};
use aptos_infallible::Mutex;
use aptos_time_service::TimeService;
use aptos_types::{jwks, jwks::rsa::RSA_JWK};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{task::JoinHandle, time::timeout};

// Test issuer names for Apple and Google
const ISSUER_APPLE: &str = "issuer_apple";
const ISSUER_GOOGLE: &str = "issuer_google";

// Maximum wait time (secs) for each test to complete
const MAX_TEST_WAIT_SECS: u64 = 10;

/// A mock JWK issuer (for testing JWK fetching and caching)
struct MockJWKIssuer {
    issuer_name: String,
    jwk_key_set: HashMap<KeyID, Arc<RSA_JWK>>,
    num_fetch_failures: Arc<Mutex<u64>>, // The number of fetch failures to simulate
}

impl MockJWKIssuer {
    pub fn new(
        issuer_name: String,
        jwk_key_set: HashMap<KeyID, Arc<RSA_JWK>>,
        num_fetch_failures: u64,
    ) -> Self {
        Self {
            issuer_name,
            jwk_key_set,
            num_fetch_failures: Arc::new(Mutex::new(num_fetch_failures)),
        }
    }
}

#[async_trait::async_trait]
impl JWKIssuerInterface for MockJWKIssuer {
    fn issuer_name(&self) -> String {
        self.issuer_name.clone()
    }

    fn issuer_jwk_url(&self) -> String {
        "".into() // The URL is not used in the mock implementation
    }

    async fn fetch_jwks(&self) -> anyhow::Result<HashMap<KeyID, Arc<RSA_JWK>>> {
        // If there are failures to simulate, decrement the counter and
        // return an error, otherwise, return the JWK key set.
        let mut num_fetch_failures = self.num_fetch_failures.lock();
        if *num_fetch_failures > 0 {
            *num_fetch_failures -= 1;
            Err(anyhow::anyhow!(
                "Simulated fetch failure for issuer {}! Failures remaining: {}",
                self.issuer_name,
                *num_fetch_failures
            ))
        } else {
            Ok(self.jwk_key_set.clone())
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn jwk_cache_updates_after_interval() {
    // Create test JWK key sets
    let apple_jwk_key_set = create_test_jwk_key_set(ISSUER_APPLE.into());
    let google_jwk_key_set = create_test_jwk_key_set(ISSUER_GOOGLE.into());

    // Create mock JWK issuers for Apple and Google
    let apple_jwk_issuer = Arc::new(MockJWKIssuer::new(
        ISSUER_APPLE.into(),
        apple_jwk_key_set.clone(),
        0,
    ));
    let google_jwk_issuer = Arc::new(MockJWKIssuer::new(
        ISSUER_GOOGLE.into(),
        google_jwk_key_set.clone(),
        0,
    ));

    // Verify that the JWK cache is updated correctly
    verify_jwk_cache_updates(
        apple_jwk_key_set,
        google_jwk_key_set,
        apple_jwk_issuer,
        google_jwk_issuer,
    )
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn jwk_cache_updates_after_failures() {
    // Create test JWK key sets
    let apple_jwk_key_set = create_test_jwk_key_set(ISSUER_APPLE.into());
    let google_jwk_key_set = create_test_jwk_key_set(ISSUER_GOOGLE.into());

    // Create mock JWK issuers that fail a few times before succeeding
    let num_fetch_failures = 3;
    let apple_jwk_issuer = Arc::new(MockJWKIssuer::new(
        ISSUER_APPLE.into(),
        apple_jwk_key_set.clone(),
        num_fetch_failures,
    ));
    let google_jwk_issuer = Arc::new(MockJWKIssuer::new(
        ISSUER_GOOGLE.into(),
        google_jwk_key_set.clone(),
        num_fetch_failures,
    ));

    // Verify that the JWK cache is updated correctly
    verify_jwk_cache_updates(
        apple_jwk_key_set,
        google_jwk_key_set,
        apple_jwk_issuer,
        google_jwk_issuer,
    )
    .await;
}

/// Creates a JWK cache and starts the refresh loops for known issuers
fn create_and_start_jwk_cache(
    apple_jwk_issuer: Arc<dyn JWKIssuerInterface + Send + Sync>,
    google_jwk_issuer: Arc<dyn JWKIssuerInterface + Send + Sync>,
    time_service: TimeService,
) -> JWKCache {
    // Create the JWK cache
    let jwk_cache = Arc::new(Mutex::new(HashMap::new()));

    // Start the JWK refresh loops for Apple and Google
    jwk_fetcher::start_jwk_refresh_loop(apple_jwk_issuer, jwk_cache.clone(), time_service.clone());
    jwk_fetcher::start_jwk_refresh_loop(google_jwk_issuer, jwk_cache.clone(), time_service.clone());

    jwk_cache
}

/// Creates a test JWK key set with a single RSA key
fn create_test_jwk_key_set(issuer_name: String) -> HashMap<KeyID, Arc<RSA_JWK>> {
    // Create several keys with different IDs
    let mut key_set = HashMap::new();
    for i in 0..5 {
        let key_id = format!("{}_key_{}", issuer_name, i);
        key_set.insert(key_id, Arc::new(jwks::insecure_test_rsa_jwk()));
    }

    key_set
}

/// Verifies that the cached resources are eventually updated
/// correctly after the resource fetcher is started.
async fn verify_jwk_cache_updates(
    apple_jwk_key_set: HashMap<KeyID, Arc<RSA_JWK>>,
    google_jwk_key_set: HashMap<KeyID, Arc<RSA_JWK>>,
    apple_jwk_issuer: Arc<dyn JWKIssuerInterface + Send + Sync>,
    google_jwk_issuer: Arc<dyn JWKIssuerInterface + Send + Sync>,
) {
    // Create the JWK cache and start the refresh loops
    let time_service = TimeService::mock();
    let jwk_cache =
        create_and_start_jwk_cache(apple_jwk_issuer, google_jwk_issuer, time_service.clone());

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
                utils::advance_time_secs(time_service.clone(), JWK_REFRESH_INTERVAL_SECS + 1).await;

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
