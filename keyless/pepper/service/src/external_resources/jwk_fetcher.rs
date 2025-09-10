// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics::JWK_FETCH_SECONDS, utils};
use anyhow::{anyhow, Result};
use aptos_infallible::Mutex;
use aptos_keyless_pepper_common::jwt::parse;
use aptos_logger::{info, warn};
use aptos_types::{jwks::rsa::RSA_JWK, keyless::test_utils::get_sample_iss};
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::Instant;

// Issuer and JWK URL constants for Apple
const ISSUER_APPLE: &str = "https://appleid.apple.com";
const JWK_URL_APPLE: &str = "https://appleid.apple.com/auth/keys";

// Issuer and JWK URL constants for Google
const ISSUER_GOOGLE: &str = "https://accounts.google.com";
const JWK_URL_GOOGLE: &str = "https://www.googleapis.com/oauth2/v3/certs";

// The interval (in seconds) at which to refresh the JWKs
const JWK_REFRESH_INTERVAL_SECS: u64 = 10;

// Useful type declarations
pub type Issuer = String;
pub type KeyID = String;
pub type JWKCache = Arc<Mutex<HashMap<Issuer, HashMap<KeyID, Arc<RSA_JWK>>>>>;

/// A lazy static regex to match auth0 URLs
static AUTH_0_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^https://[a-zA-Z0-9-]+\.us\.auth0\.com/$").unwrap());

/// A lazy static regex to match cognito URLs
static COGNITO_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^https://cognito-idp\.[a-zA-Z0-9-_]+\.amazonaws\.com/[a-zA-Z0-9-_]+$").unwrap()
});

/// Fetches the JWKs from the given URL
async fn fetch_jwks(jwk_url: &str) -> Result<HashMap<KeyID, Arc<RSA_JWK>>> {
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
pub async fn get_federated_jwk(jwt: &str) -> Result<Arc<RSA_JWK>> {
    // Parse the JWT to extract the issuer and key ID
    let payload = parse(jwt)?;
    let jwt_issuer = payload.claims.iss;
    let jwt_key_id: String = match payload.header.kid {
        Some(kid) => kid,
        None => return Err(anyhow!("No key ID (kid) found on JWT header!")),
    };

    // Fetch the keys for the issuer
    let keys = if jwt_issuer.eq("test.federated.oidc.provider") {
        let test_jwk = include_str!("../../../../../types/src/jwks/rsa/secure_test_jwk.json");
        parse_jwks(test_jwk).expect("The test JWK should parse successfully!")
    } else if AUTH_0_REGEX.is_match(&jwt_issuer) {
        let jwk_url = format!("{}.well-known/jwks.json", &jwt_issuer);
        fetch_jwks(&jwk_url).await?
    } else if COGNITO_REGEX.is_match(&jwt_issuer) {
        let jwk_url = format!("{}/.well-known/jwks.json", &jwt_issuer);
        fetch_jwks(&jwk_url).await?
    } else {
        return Err(anyhow!("Unsupported federated issuer: {}", jwt_issuer));
    };

    // Fetch the key for the given key ID
    let key = keys
        .get(&jwt_key_id)
        .ok_or_else(|| anyhow!("unknown kid: {}", jwt_key_id))?;
    Ok(key.clone())
}

/// Inserts the test JWK into the JWT cache
fn insert_test_jwk(jwk_cache: JWKCache) {
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
pub fn start_jwk_fetchers() -> JWKCache {
    // Create the JWK cache
    let jwk_cache = Arc::new(Mutex::new(HashMap::new()));

    // Insert the test JWK. This is required for the automated end-to-end
    // integration tests that run as a part of testing pipeline.
    insert_test_jwk(jwk_cache.clone());

    // Start the JWK refresh loops for known issuers
    start_jwk_refresh_loop(
        ISSUER_GOOGLE.into(),
        JWK_URL_GOOGLE.into(),
        jwk_cache.clone(),
    );
    start_jwk_refresh_loop(ISSUER_APPLE.into(), JWK_URL_APPLE.into(), jwk_cache.clone());

    // Return the JWK cache
    jwk_cache
}

/// Starts a background task that periodically fetches and caches the JWKs from the given URL
fn start_jwk_refresh_loop(issuer: String, jwk_url: String, jwk_cache: JWKCache) {
    tokio::spawn(async move {
        loop {
            // Fetch the JWKs from the URL
            let time_now = Instant::now();
            let fetch_result = fetch_jwks(&jwk_url).await;
            let fetch_time = time_now.elapsed();

            // Process the fetch result
            match &fetch_result {
                Ok(key_set) => {
                    // Log the successful fetch
                    info!(
                        issuer = issuer,
                        jwk_url = jwk_url,
                        "Successfully fetched the JWK! Issuer: {}, Key set: {:?}",
                        issuer,
                        key_set
                    );

                    // Update the cache
                    jwk_cache.lock().insert(issuer.clone(), key_set.clone());
                },
                Err(error) => {
                    warn!(
                        issuer = issuer,
                        jwk_url = jwk_url,
                        "Failed to fetch the JWK! Issuer: {}, Error: {}",
                        issuer,
                        error
                    );
                },
            }

            // Update the fetch metrics
            let succeeded = fetch_result.is_ok();
            JWK_FETCH_SECONDS
                .with_label_values(&[&issuer, &succeeded.to_string()])
                .observe(fetch_time.as_secs_f64());

            // Sleep until the next refresh interval
            let refresh_interval = Duration::from_secs(JWK_REFRESH_INTERVAL_SECS);
            tokio::time::sleep(refresh_interval).await;
        }
    });
}
