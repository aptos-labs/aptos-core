// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::counters::OBSERVATION_SECONDS;
use anyhow::Result;
use aptos_channels::aptos_channel;
use aptos_logger::{debug, info};
use aptos_types::jwks::{jwk::JWK, Issuer};
use futures::{FutureExt, StreamExt};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::{sync::oneshot, task::JoinHandle, time::MissedTickBehavior};

#[derive(Serialize, Deserialize)]
struct OpenIDConfiguration {
    issuer: String,
    jwks_uri: String,
}

#[derive(Serialize, Deserialize)]
struct JWKsResponse {
    keys: Vec<serde_json::Value>,
}

/// Given an Open ID configuration URL, fetch its JWKs.
pub async fn fetch_jwks(my_addr: AccountAddress, config_url: Vec<u8>) -> Result<Vec<JWK>> {
    let maybe_url = String::from_utf8(config_url);
    let config_url = maybe_url?;
    let client = reqwest::Client::new();
    let OpenIDConfiguration { jwks_uri, .. } =
        client.get(config_url.as_str()).send().await?.json().await?;
    let JWKsResponse { keys } = client.get(jwks_uri.as_str()).send().await?.json().await?;
    let jwks = keys.into_iter().map(JWK::from).collect();
    Ok(jwks)
}

/// A process thread that periodically fetch JWKs of a provider and push it back to JWKManager.
pub struct JWKObserver {
    close_tx: oneshot::Sender<()>,
    join_handle: JoinHandle<()>,
}

impl JWKObserver {
    pub fn spawn(
        epoch: u64,
        my_addr: AccountAddress,
        issuer: Issuer,
        config_url: Vec<u8>,
        fetch_interval: Duration,
        observation_tx: aptos_channel::Sender<(), (Issuer, Vec<JWK>)>,
    ) -> Self {
        let (close_tx, close_rx) = oneshot::channel();
        let join_handle = tokio::spawn(Self::start(
            fetch_interval,
            my_addr,
            issuer.clone(),
            config_url.clone(),
            observation_tx,
            close_rx,
        ));
        info!(
            epoch = epoch,
            issuer = String::from_utf8(issuer).ok(),
            config_url = String::from_utf8(config_url).ok(),
            "JWKObserver spawned."
        );
        Self {
            close_tx,
            join_handle,
        }
    }

    async fn start(
        fetch_interval: Duration,
        my_addr: AccountAddress,
        issuer: Issuer,
        open_id_config_url: Vec<u8>,
        observation_tx: aptos_channel::Sender<(), (Issuer, Vec<JWK>)>,
        close_rx: oneshot::Receiver<()>,
    ) {
        let issuer_str =
            String::from_utf8(issuer.clone()).unwrap_or_else(|_e| "UNKNOWN_ISSUER".to_string());
        let mut interval = tokio::time::interval(fetch_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut close_rx = close_rx.into_stream();
        loop {
            tokio::select! {
                _ = interval.tick().fuse() => {
                    let timer = Instant::now();
                    let result = fetch_jwks(my_addr, open_id_config_url.clone()).await;
                    let secs = timer.elapsed().as_secs_f64();
                    debug!(issuer = issuer_str, "observe_result={:?}", result);
                    if let Ok(mut jwks) = result {
                        OBSERVATION_SECONDS.with_label_values(&[&issuer_str, "ok"]).observe(secs);
                        jwks.sort();
                        let _ = observation_tx.push((), (issuer.clone(), jwks));
                    } else {
                        OBSERVATION_SECONDS.with_label_values(&[&issuer_str, "err"]).observe(secs);
                    }
                },
                _ = close_rx.select_next_some() => {
                    break;
                }
            }
        }
    }

    pub async fn shutdown(self) {
        let Self {
            close_tx,
            join_handle,
        } = self;
        let _ = close_tx.send(());
        let _ = join_handle.await;
    }
}

#[ignore]
#[tokio::test]
async fn test_fetch_real_jwks() {
    let jwks = fetch_jwks(
        AccountAddress::ZERO,
        "https://www.facebook.com/.well-known/openid-configuration/"
            .as_bytes()
            .to_vec(),
    )
    .await
    .unwrap();
    println!("{:?}", jwks);
}
