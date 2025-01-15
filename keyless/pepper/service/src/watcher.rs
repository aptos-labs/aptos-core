// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, ensure, Result};
use aptos_infallible::RwLock;
use aptos_logger::{debug, info, warn};
use serde::de::DeserializeOwned;
use std::{sync::Arc, time::Duration};

pub async fn fetch_and_cache_resource<T: DeserializeOwned>(
    resource_url: &str,
    resource_holder: &RwLock<Option<T>>,
) -> Result<()> {
    let resource = reqwest::get(resource_url).await?.json::<T>().await?;
    *resource_holder.write() = Some(resource);
    Ok(())
}

pub fn start_external_resource_refresh_loop<
    T: DeserializeOwned + ExternalResource + Send + Sync + 'static,
>(
    url: &str,
    refresh_interval: Duration,
    local_cache: Arc<RwLock<Option<T>>>,
) {
    info!(
        "Starting external resource refresh loop for {}",
        T::resource_name()
    );
    let url = url.to_string();
    let _handle = tokio::spawn(async move {
        loop {
            let result = fetch_and_cache_resource(&url, local_cache.as_ref()).await;
            match result {
                Ok(_vk) => {
                    debug!("fetch_and_cache_resource {} succeeded.", T::resource_name());
                },
                Err(e) => {
                    warn!(
                        "fetch_and_cache_resource {} failed: {}",
                        T::resource_name(),
                        e
                    );
                },
            }

            tokio::time::sleep(refresh_interval).await;
        }
    });
}

pub trait ExternalResource {
    fn resource_name() -> String;
}

pub fn unhexlify_api_bytes(api_output: &str) -> Result<Vec<u8>> {
    ensure!(api_output.len() >= 2);
    let lower = api_output.to_lowercase();
    ensure!(&lower[0..2] == "0x");
    let bytes = hex::decode(&lower[2..])
        .map_err(|e| anyhow!("unhexlify_api_bytes() failed at decoding: {e}"))?;
    Ok(bytes)
}

#[test]
fn test_unhexlify_api_bytes() {
    assert_eq!(
        vec![0x00_u8, 0x01, 0xFF],
        unhexlify_api_bytes("0x0001ff").unwrap()
    );
    assert!(unhexlify_api_bytes("0x").unwrap().is_empty());
    assert!(unhexlify_api_bytes("0001ff").is_err());
    assert!(unhexlify_api_bytes("0x0001fg").is_err());
    assert!(unhexlify_api_bytes("000").is_err());
    assert!(unhexlify_api_bytes("0").is_err());
    assert!(unhexlify_api_bytes("").is_err());
}
