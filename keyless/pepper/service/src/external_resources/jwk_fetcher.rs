// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    external_resources::jwk_types::{
        FederatedJWKIssuer, FederatedJWKIssuerInterface, FederatedJWKs, JWKCache, JWKIssuer,
        JWKIssuerInterface, KeyID,
    },
    metrics, utils,
};
use anyhow::{anyhow, Result};
use aptos_infallible::Mutex;
use aptos_keyless_pepper_common::jwt::parse;
use aptos_logger::{info, warn};
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{jwks::rsa::RSA_JWK, keyless::test_utils::get_sample_iss};
use serde_json::Value;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::Instant;
// TODO: at some point, we should try to merge the JWK and resource fetcher code

// Issuer and JWK URL constants for Apple
const ISSUER_APPLE: &str = "https://appleid.apple.com";
const JWK_URL_APPLE: &str = "https://appleid.apple.com/auth/keys";

// Issuer and JWK URL constants for Google
const ISSUER_GOOGLE: &str = "https://accounts.google.com";
const JWK_URL_GOOGLE: &str = "https://www.googleapis.com/oauth2/v3/certs";

// The interval (in seconds) at which to refresh the JWKs
pub const JWK_REFRESH_INTERVAL_SECS: u64 = 10;

// The frequency at which to log the JWK refresh status (per loop iteration)
const JWK_REFRESH_LOOP_LOG_FREQUENCY: u64 = 6; // e.g., 6 * 10s (per loop) = 60s per log

// Auth0 federated JWK constants
pub const AUTH0_ISSUER_NAME: &str = "auth0";
pub const AUTH0_REGEX_STR: &str = r"^https://[a-zA-Z0-9-]+\.us\.auth0\.com/$";
pub const AUTH0_JWK_URL_SUFFIX: &str = ".well-known/jwks.json";

// Cognito federated JWK constants
pub const COGNITO_ISSUER_NAME: &str = "cognito";
pub const COGNITO_REGEX_STR: &str =
    r"^https://cognito-idp\.[a-zA-Z0-9-_]+\.amazonaws\.com/[a-zA-Z0-9-_]+$";
pub const COGNITO_JWK_URL_SUFFIX: &str = "/.well-known/jwks.json";

/// Creates and initializes the federated JWKs map
fn initialize_federated_jwks() -> FederatedJWKs<FederatedJWKIssuer> {
    let mut federated_jwks = Vec::new();

    // Add the Auth0 federated JWKs
    let auth0_jwk = FederatedJWKIssuer::new(
        AUTH0_ISSUER_NAME.into(),
        AUTH0_JWK_URL_SUFFIX.into(),
        AUTH0_REGEX_STR.into(),
    );
    federated_jwks.push(auth0_jwk);

    // Add the cognito federated JWKs
    let cognito_jwk = FederatedJWKIssuer::new(
        COGNITO_ISSUER_NAME.into(),
        COGNITO_JWK_URL_SUFFIX.into(),
        COGNITO_REGEX_STR.into(),
    );
    federated_jwks.push(cognito_jwk);

    // Return the federated JWKs
    FederatedJWKs::new(federated_jwks)
}

/// Fetches the JWKs from the given URL
pub async fn fetch_jwks(jwk_url: &str) -> Result<HashMap<KeyID, Arc<RSA_JWK>>> {
    // Create the request client
    let client = utils::create_request_client();

    // Fetch the JWKs from the URL
    let response = client
        .get(jwk_url)
        .send()
        .await
        .map_err(|error| anyhow!("Failed to fetch JWKs from {}! Error: {}", jwk_url, error))?;

    // Extract the response text
    let response_text = response.text().await.map_err(|error| {
        anyhow!(
            "Failed to extract response text from {}! Error: {}",
            jwk_url,
            error
        )
    })?;

    // Parse the JWKs from the response text
    parse_jwks(&response_text)
}

/// Returns a cached RSA JWK for the given issuer and key ID
pub fn get_cached_jwk_as_rsa(
    issuer: &String,
    key_id: &String,
    jwk_cache: JWKCache,
) -> Result<Arc<RSA_JWK>> {
    // Get the key set for the issuer
    let jwk_cache = jwk_cache.lock();
    let key_set = jwk_cache
        .get(issuer)
        .ok_or_else(|| anyhow!("Failed to get cached RSA JWK! Unknown issuer: {}", issuer))?;

    // Get the key for the given key ID
    let key = key_set
        .get(key_id)
        .ok_or_else(|| anyhow!("Failed to get cached RSA JWK! Unknown key ID: {}", key_id))?;

    Ok(key.clone())
}

/// Fetches the federated JWK for the given JWT
pub async fn get_federated_jwk<T: FederatedJWKIssuerInterface + Clone>(
    jwt: &str,
    federated_jwks: FederatedJWKs<T>,
) -> Result<Arc<RSA_JWK>> {
    // Parse the JWT to extract the issuer and key ID
    let payload = parse(jwt)?;
    let jwt_issuer = payload.claims.iss;
    let jwt_key_id: String = match payload.header.kid {
        Some(kid) => kid,
        None => return Err(anyhow!("No key ID (kid) found on JWT header!")),
    };

    // Fetch the keys for the issuer
    let keys = if jwt_issuer.eq("test.federated.oidc.provider") {
        // Use the test JWK for the test issuer
        let test_jwk = include_str!("../../../../../types/src/jwks/rsa/secure_test_jwk.json");
        parse_jwks(test_jwk).expect("The test JWK should parse successfully!")
    } else {
        // Fetch the JWKs for the issuer
        let mut found_jwks = None;
        for federated_issuer in federated_jwks.get_issuers() {
            if federated_issuer.regex().is_match(&jwt_issuer) {
                // Fetch the jwks from the URL
                let fetched_jwks = federated_issuer.fetch_jwks(jwt_issuer.clone()).await?;

                // Update the found jwks
                found_jwks = Some(fetched_jwks);
                break;
            }
        }

        // Ensure the issuer was found
        match found_jwks {
            Some(jwks) => jwks,
            None => return Err(anyhow!("Unsupported federated issuer: {}", jwt_issuer)),
        }
    };

    // Fetch the key for the given key ID
    let key = keys
        .get(&jwt_key_id)
        .ok_or_else(|| anyhow!("Unknown kid: {}", jwt_key_id))?;
    Ok(key.clone())
}

/// Inserts the test JWK into the JWT cache
pub fn insert_test_jwk(jwk_cache: JWKCache) {
    let test_jwk = include_str!("../../../../../types/src/jwks/rsa/secure_test_jwk.json");
    let parsed_jwk = parse_jwks(test_jwk).expect("The test JWK should parse successfully!");
    jwk_cache.lock().insert(get_sample_iss(), parsed_jwk);
}

/// Parses the JWKs from the given response text
fn parse_jwks(response_text: &str) -> Result<HashMap<KeyID, Arc<RSA_JWK>>> {
    // Parse the response text into a JSON value
    let response_json_value = serde_json::from_str::<Value>(response_text)
        .map_err(|error| anyhow!("Failed to parse response json! Error: {}", error))?;

    // Extract the "keys" array from the JSON value
    let keys: &Vec<Value> = response_json_value
        .get("keys")
        .ok_or_else(|| anyhow!("Failed to parse JWK json: \"keys\" entry not found!"))?
        .as_array()
        .ok_or_else(|| anyhow!("Failed to parse JWK json: \"keys\" entry not an array!"))?;

    // Parse each key, and filter out unsupported keys
    let key_map: HashMap<KeyID, Arc<RSA_JWK>> = keys
        .iter()
        .filter_map(|jwk_val| match RSA_JWK::try_from(jwk_val) {
            Ok(rsa_jwk) => {
                if rsa_jwk.e == "AQAB" {
                    Some((rsa_jwk.kid.clone(), Arc::new(rsa_jwk)))
                } else {
                    warn!("Unsupported RSA modulus for jwk: {}", jwk_val);
                    None
                }
            },
            Err(error) => {
                warn!("Error while parsing JWK: {}! {}", jwk_val, error);
                None
            },
        })
        .collect();

    Ok(key_map)
}

/// Starts the JWK refresh loops for known issuers. Note: we currently
/// hardcode the known issuers here, instead of fetching them from on-chain
/// configs. This is a security measure to ensure the pepper service only
/// trusts a small set of known issuers, with deterministic and immutable
/// JWK URLs. Otherwise, if these values were fetched from on-chain configs,
/// an attacker who compromises governance could change these values to
/// point to a malicious issuer (or JWK URL).
pub fn start_jwk_fetchers(
    jwk_issuers_override: Vec<JWKIssuer>,
) -> (JWKCache, FederatedJWKs<FederatedJWKIssuer>) {
    // Create the JWK cache
    let jwk_cache = Arc::new(Mutex::new(HashMap::new()));

    // Insert the test JWK. This is required for the automated end-to-end
    // integration tests that run as a part of testing pipeline.
    insert_test_jwk(jwk_cache.clone());

    // Create the time service
    let time_service = TimeService::real();

    // Create the known issuers for Google and Apple
    let default_issuers = vec![
        JWKIssuer::new(ISSUER_GOOGLE.into(), JWK_URL_GOOGLE.into()),
        JWKIssuer::new(ISSUER_APPLE.into(), JWK_URL_APPLE.into()),
    ];
    let jwk_issuer_map: HashMap<String, Arc<JWKIssuer>> = default_issuers
        .into_iter()
        .chain(jwk_issuers_override)
        .map(|issuer| (issuer.issuer_name(), Arc::new(issuer)))
        .collect();

    // Start the JWK refresh loops for known issuers
    for (_, jwk_issuer) in jwk_issuer_map {
        start_jwk_refresh_loop(jwk_issuer, jwk_cache.clone(), time_service.clone());
    }

    // Initialize the federated JWKs
    let federated_jwks = initialize_federated_jwks();

    // Return the JWK cache and federated JWKs
    (jwk_cache, federated_jwks)
}

/// Starts a background task that periodically fetches and caches the JWKs from the given issuer
pub fn start_jwk_refresh_loop(
    jwk_issuer: Arc<dyn JWKIssuerInterface + Send + Sync>,
    jwk_cache: JWKCache,
    time_service: TimeService,
) {
    // Log the start of the task for the issuer
    let issuer_name = jwk_issuer.issuer_name();
    let issuer_jwk_url = jwk_issuer.issuer_jwk_url();
    info!(
        "Starting the JWK refresh loop for {}, URL: {}!",
        issuer_name, issuer_jwk_url
    );

    // Start the task
    tokio::spawn(async move {
        let mut loop_iteration_counter: u64 = 0;

        loop {
            // Fetch the JWKs from the URL
            let time_now = Instant::now();
            let fetch_result = jwk_issuer.fetch_jwks().await;
            let fetch_time = time_now.elapsed();

            // Process the fetch result
            match &fetch_result {
                Ok(key_set) => {
                    // Log the successful fetch
                    if loop_iteration_counter % JWK_REFRESH_LOOP_LOG_FREQUENCY == 0 {
                        info!(
                            "Successfully fetched the JWK in {:?}! Issuer: {}, URL: {}, Key set: {:?}",
                            fetch_time,
                            issuer_jwk_url,
                            issuer_name,
                            key_set
                        )
                    }

                    // Update the cache
                    jwk_cache
                        .lock()
                        .insert(issuer_name.clone(), key_set.clone());
                },
                Err(error) => {
                    warn!(
                        "Failed to fetch the JWK in {:?}! Issuer: {}, URL: {}, Error: {}",
                        fetch_time, issuer_jwk_url, issuer_name, error
                    );
                },
            }

            // Update the fetch metrics
            metrics::update_jwk_fetch_metrics(&issuer_name, fetch_result.is_ok(), fetch_time);

            // Increment the loop iteration counter
            loop_iteration_counter = loop_iteration_counter.wrapping_add(1);

            // Sleep until the next refresh interval
            let refresh_interval = Duration::from_secs(JWK_REFRESH_INTERVAL_SECS);
            time_service.sleep(refresh_interval).await;
        }
    });
}
