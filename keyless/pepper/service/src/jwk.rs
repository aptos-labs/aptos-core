// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics::JWK_FETCH_SECONDS, Issuer, KeyID};
use anyhow::{anyhow, Result};
use aptos_keyless_pepper_common::jwt::parse;
use aptos_logger::warn;
use aptos_types::jwks::rsa::RSA_JWK;
use dashmap::DashMap;
use jsonwebtoken::DecodingKey;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::Value;
use std::{sync::Arc, time::Duration};
use tokio::time::Instant;

static AUTH_0_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^https://[a-zA-Z0-9-]+\.us\.auth0\.com/$").unwrap());

/// The JWK in-mem cache.
pub static DECODING_KEY_CACHE: Lazy<DashMap<Issuer, DashMap<KeyID, Arc<RSA_JWK>>>> =
    Lazy::new(DashMap::new);

pub async fn get_federated_jwk(jwt: &str) -> Result<Arc<RSA_JWK>> {
    let payload = parse(jwt)?;

    let jwt_kid: String = match payload.header.kid {
        Some(kid) => kid,
        None => return Err(anyhow!("no kid found on jwt header")),
    };

    // Check if it is a test iss
    let keys = if payload.claims.iss.eq("test.federated.oidc.provider") {
        let test_jwk = include_str!("../../../../types/src/jwks/rsa/secure_test_jwk.json");
        parse_jwks(test_jwk).expect("test jwk should parse")
    } else if AUTH_0_REGEX.is_match(&payload.claims.iss) {
        let jwk_url = format!("{}.well-known/jwks.json", &payload.claims.iss);
        fetch_jwks(&jwk_url).await?
    } else {
        return Err(anyhow!("not a federated iss"));
    };

    let key = keys
        .get(&jwt_kid)
        .ok_or_else(|| anyhow!("unknown kid: {}", jwt_kid))?;
    Ok(key.clone())
}

/// Send a request to a JWK endpoint and return its JWK map.
pub async fn fetch_jwks(jwk_url: &str) -> Result<DashMap<KeyID, Arc<RSA_JWK>>> {
    let response = reqwest::get(jwk_url)
        .await
        .map_err(|e| anyhow!("jwk fetch error: {}", e))?;
    let text = response
        .text()
        .await
        .map_err(|e| anyhow!("error while getting response as text: {}", e))?;
    parse_jwks(&text)
}

pub fn parse_jwks(text: &str) -> Result<DashMap<KeyID, Arc<RSA_JWK>>> {
    let endpoint_response_val = serde_json::from_str::<Value>(text)
        .map_err(|e| anyhow!("error while parsing json: {}", e))?;

    let keys: &Vec<Value> = endpoint_response_val
        .get("keys")
        .ok_or_else(|| anyhow!("Error while parsing jwk json: \"keys\" not found"))?
        .as_array()
        .ok_or_else(|| anyhow!("Error while parsing jwk json: \"keys\" not array"))?;
    let key_map: DashMap<KeyID, Arc<RSA_JWK>> = keys
        .iter()
        .filter_map(|jwk_val| match RSA_JWK::try_from(jwk_val) {
            Ok(jwk) => {
                if jwk.e == "AQAB" {
                    Some((jwk.kid.clone(), Arc::new(jwk)))
                } else {
                    warn!("Unsupported RSA modulus for jwk: {}", jwk_val);
                    None
                }
            },
            Err(e) => {
                warn!("error while parsing jwk {}: {e}", jwk_val);
                None
            },
        })
        .collect();
    Ok(key_map)
}

pub fn start_jwk_refresh_loop(issuer: &str, jwk_url: &str, refresh_interval: Duration) {
    let issuer = issuer.to_string();
    let jwk_url = jwk_url.to_string();
    let _handle = tokio::spawn(async move {
        loop {
            let timer = Instant::now();
            let fetch_result = fetch_jwks(jwk_url.as_str()).await;
            let fetch_time = timer.elapsed();
            let succeeded = fetch_result.is_ok();
            JWK_FETCH_SECONDS
                .with_label_values(&[issuer.as_str(), succeeded.to_string().as_str()])
                .observe(fetch_time.as_secs_f64());
            match fetch_result {
                Ok(key_set) => {
                    DECODING_KEY_CACHE.insert(issuer.clone(), key_set.clone());
                },
                Err(msg) => {
                    warn!(
                        issuer = issuer,
                        jwk_url = jwk_url,
                        "error fetching JWK: {}",
                        msg
                    );
                },
            }
            tokio::time::sleep(refresh_interval).await;
        }
    });
}

pub fn cached_decoding_key_as_rsa(issuer: &String, kid: &String) -> Result<Arc<RSA_JWK>> {
    let key_set = DECODING_KEY_CACHE
        .get(issuer)
        .ok_or_else(|| anyhow!("unknown issuer: {}", issuer))?;
    let key = key_set
        .get(kid)
        .ok_or_else(|| anyhow!("unknown kid: {}", kid))?;
    Ok(key.clone())
}

pub fn cached_decoding_key(issuer: &String, kid: &String) -> Result<DecodingKey> {
    let key = cached_decoding_key_as_rsa(issuer, kid)?;
    let decoding_key = DecodingKey::from_rsa_components(&key.n, &key.e)?;
    Ok(decoding_key)
}
