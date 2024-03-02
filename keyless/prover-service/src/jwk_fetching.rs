use anyhow::{anyhow, Result};
use aptos_types::jwks::rsa::RSA_JWK;
use dashmap::DashMap;

use tracing::{info, warn};
use once_cell::sync::Lazy;
use std::{sync::Arc, thread::sleep, time::Duration};
use serde_json::*;

use crate::config::OidcProvider;

pub type Issuer = String;
pub type KeyID = String;



// TODO: this is a duplicate of the jwk fetching in the pepper service, with changes b/c the
// DecodingKey type that the pepper service uses is too opaque to use here. We should unify.

/// The JWK in-mem cache.
pub static DECODING_KEY_CACHE: Lazy<DashMap<Issuer, DashMap<KeyID, Arc<RSA_JWK>>>> =
    Lazy::new(DashMap::new);

/// Send a request to a JWK endpoint and return its JWK map.
pub async fn fetch_jwks(jwk_url: &str) -> Result<DashMap<KeyID, Arc<RSA_JWK>>> {
    let response = reqwest::get(jwk_url)
        .await
        .map_err(|e| anyhow!("jwk fetch error: {}", e))?;
    let text = response
        .text()
        .await
        .map_err(|e| anyhow!("error while getting response as text: {}", e))?;
    let endpoint_response_val = serde_json::from_str::<Value>(text.as_str())
        .map_err(|e| anyhow!("error while parsing json: {}", e))?;
    let keys : &Vec<Value> = endpoint_response_val["keys"]
        .as_array()
        .ok_or(anyhow!("Error while parsing jwk json: \"keys\" not found"))?;
    let key_map: DashMap<KeyID, Arc<RSA_JWK>> = keys
        .iter()
        .filter_map(
            |jwk_val| match RSA_JWK::try_from(jwk_val) {
                Ok(jwk) => {
                    if jwk.e != "AQAB" {
                        warn!("Unsupported RSA modulus for jwk: {}", jwk_val);
                        None
                    } else {
                        Some((jwk.kid.clone(), Arc::new(jwk)))
                    }
                }
                Err(e) => {
                    warn!("error while parsing for jwk {}: {e}", jwk_val);
                    None
                },
            },
        )
        .collect();
    Ok(key_map)
}

pub async fn populate_jwk_cache(issuer: &str, jwk_url: &str) {
    let issuer = issuer.to_string();
    let jwk_url = jwk_url.to_string();

    match fetch_jwks(jwk_url.as_str()).await {
        Ok(key_set) => {
            let num_keys = key_set.len();
            DECODING_KEY_CACHE.insert(issuer.clone(), key_set);
            info!(
                "Updated key set of issuer {}. Num of keys: {}.",
                issuer, num_keys
                );
        },
        Err(msg) => {
            warn!("{}", msg);
        },
    }
}

pub fn start_jwk_refresh_loop(issuer: &str, jwk_url: &str, refresh_interval: Duration) {
    let issuer = issuer.to_string();
    let jwk_url = jwk_url.to_string();
    let _handle = tokio::spawn(async move {
        loop {
            match fetch_jwks(jwk_url.as_str()).await {
                Ok(key_set) => {
                    let num_keys = key_set.len();
                    DECODING_KEY_CACHE.insert(issuer.clone(), key_set);
                    info!(
                        "Updated key set of issuer {}. Num of keys: {}.",
                        issuer, num_keys
                    );
                },
                Err(msg) => {
                    warn!("{}", msg);
                },
            }
            sleep(refresh_interval);
        }
    });
}

pub fn cached_decoding_key(issuer: &String, kid: &String) -> Result<Arc<RSA_JWK>> {
    info!("current cache: {:?}", DECODING_KEY_CACHE);
    let key_set = DECODING_KEY_CACHE
        .get(issuer)
        .ok_or_else(|| anyhow!("unknown issuer: {}", issuer))?;
    let key = key_set
        .get(kid)
        .ok_or_else(|| anyhow!("unknown kid: {}", kid))?;
    Ok(key.clone())
}


pub async fn init_jwk_fetching(
    oidc_providers: &Vec<OidcProvider>,
    jwk_refresh_rate: Duration
    ) {
    for provider in oidc_providers { 
        // Do initial jwk cache population non-async, so that we don't handle requests before this is
        // populated
        populate_jwk_cache(
            &provider.iss,
            &provider.endpoint_url
            ).await;

        // init jwk polling job for this provider
        start_jwk_refresh_loop(
            &provider.iss,
            &provider.endpoint_url,
            jwk_refresh_rate
            );
    }
}



