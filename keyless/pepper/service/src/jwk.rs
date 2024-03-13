// Copyright © Aptos Foundation

use crate::{Issuer, KeyID};
use anyhow::{anyhow, Result};
use dashmap::DashMap;
use jsonwebtoken::{
    jwk::{Jwk, JwkSet},
    DecodingKey,
};
use log::{info, warn};
use once_cell::sync::Lazy;
use std::{sync::Arc, time::Duration};

/// The JWK in-mem cache.
pub static DECODING_KEY_CACHE: Lazy<DashMap<Issuer, DashMap<KeyID, Arc<DecodingKey>>>> =
    Lazy::new(DashMap::new);

/// Send a request to a JWK endpoint and return its JWK map.
pub async fn fetch_jwks(jwk_url: &str) -> Result<DashMap<KeyID, Arc<DecodingKey>>> {
    let response = reqwest::get(jwk_url)
        .await
        .map_err(|e| anyhow!("jwk fetch error: {}", e))?;
    let text = response
        .text()
        .await
        .map_err(|e| anyhow!("error while getting response as text: {}", e))?;
    let JwkSet { keys } = serde_json::from_str(text.as_str())
        .map_err(|e| anyhow!("error while parsing json: {}", e))?;
    let key_map: DashMap<KeyID, Arc<DecodingKey>> = keys
        .into_iter()
        .filter_map(
            |jwk| match (&jwk.common.key_id, DecodingKey::from_jwk(&jwk)) {
                (Some(kid), Ok(key)) => Some((kid.clone(), Arc::new(key))),
                (Some(kid), Err(e)) => {
                    warn!("error while parsing for kid {kid}: {e}");
                    None
                },
                (None, _) => {
                    warn!("Ignoring a kid-less jwk: {jwk:?}");
                    None
                },
            },
        )
        .collect();
    Ok(key_map)
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
            tokio::time::sleep(refresh_interval).await;
        }
    });
}

pub fn cached_decoding_key(issuer: &String, kid: &String) -> Result<Arc<DecodingKey>> {
    let test_jwk = r#"{
        "kid": "test_jwk",
        "kty": "RSA",
        "alg": "RS256",
        "use": "sig",
        "n": "6S7asUuzq5Q_3U9rbs-PkDVIdjgmtgWreG5qWPsC9xXZKiMV1AiV9LXyqQsAYpCqEDM3XbfmZqGb48yLhb_XqZaKgSYaC_h2DjM7lgrIQAp9902Rr8fUmLN2ivr5tnLxUUOnMOc2SQtr9dgzTONYW5Zu3PwyvAWk5D6ueIUhLtYzpcB-etoNdL3Ir2746KIy_VUsDwAM7dhrqSK8U2xFCGlau4ikOTtvzDownAMHMrfE7q1B6WZQDAQlBmxRQsyKln5DIsKv6xauNsHRgBAKctUxZG8M4QJIx3S6Aughd3RZC4Ca5Ae9fd8L8mlNYBCrQhOZ7dS0f4at4arlLcajtw",
        "e": "AQAB"
    }"#;
    if kid.eq("test_jwk") {
        let key = serde_json::from_str::<Jwk>(test_jwk)
            .map_err(|e| anyhow!("error while parsing json: {}", e))?;
        let decoding_key = DecodingKey::from_jwk(&key)?;
        return Ok(Arc::new(decoding_key));
    }
    let key_set = DECODING_KEY_CACHE
        .get(issuer)
        .ok_or_else(|| anyhow!("unknown issuer: {}", issuer))?;
    let key = key_set
        .get(kid)
        .ok_or_else(|| anyhow!("unknown kid: {}", kid))?;
    Ok(key.clone())
}
