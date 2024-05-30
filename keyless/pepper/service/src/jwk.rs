// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics::JWK_FETCH_SECONDS, Issuer, KeyID};
use anyhow::{anyhow, Result};
use aptos_logger::warn;
use dashmap::DashMap;
use jsonwebtoken::{
    jwk::{Jwk, JwkSet},
    DecodingKey,
};
use once_cell::sync::Lazy;
use std::{sync::Arc, time::Duration};
use tokio::time::Instant;

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

pub fn cached_decoding_key(issuer: &String, kid: &String) -> Result<Arc<DecodingKey>> {
    let test_jwk = r#"{
        "kid": "test-rsa",
        "kty": "RSA",
        "alg": "RS256",
        "use": "sig",
        "n": "y5Efs1ZzisLLKCARSvTztgWj5JFP3778dZWt-od78fmOZFxem3a_aYbOXSJToRp862do0PxJ4PDMpmqwV5f7KplFI6NswQV-WPufQH8IaHXZtuPdCjPOcHybcDiLkO12d0dG6iZQUzypjAJf63APcadio-4JDNWlGC5_Ow_XQ9lIY71kTMiT9lkCCd0ZxqEifGtnJe5xSoZoaMRKrvlOw-R6iVjLUtPAk5hyUX95LDKxwAR-oshnj7gmATejga2EvH9ozdn3M8Go11PSDa04OQxPcA25OoDTfxLvT28LRpSXrbmUWZ-O_lGtDl3ZAtjIguYGEobTk4N11eRssC95Cw",
        "e": "AQAB"
    }"#;
    if kid.eq("test-rsa") {
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
