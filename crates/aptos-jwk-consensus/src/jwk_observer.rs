// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::counters::OBSERVATION_SECONDS;
use anyhow::{anyhow, Result};
use aptos_channels::aptos_channel;
use aptos_logger::{debug, info};
use aptos_types::jwks::{Issuer, jwk::JWK};
use futures::{FutureExt, StreamExt};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::{sync::oneshot, task::JoinHandle, time::MissedTickBehavior};
use aptos_jwk_utils::{fetch_jwks_from_jwks_uri, fetch_jwks_uri_from_openid_config};

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
        issuer_bytes: Issuer,
        open_id_config_url: Vec<u8>,
        observation_tx: aptos_channel::Sender<(), (Issuer, Vec<JWK>)>,
        close_rx: oneshot::Receiver<()>,
    ) {
        let issuer =
            String::from_utf8(issuer_bytes.clone()).unwrap_or_else(|_e| "UNKNOWN_ISSUER".to_string());
        let mut interval = tokio::time::interval(fetch_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut close_rx = close_rx.into_stream();
        let my_addr = if cfg!(feature = "smoke-test") {
            Some(my_addr)
        } else {
            None
        };

        loop {
            tokio::select! {
                _ = interval.tick().fuse() => {
                    let timer = Instant::now();
                    let result = fetch_jwks(issuer.as_str(), my_addr).await;
                    debug!(issuer = issuer, "observe_result={:?}", result);
                    let secs = timer.elapsed().as_secs_f64();
                    if let Ok(mut jwks) = result {
                        OBSERVATION_SECONDS.with_label_values(&[&issuer, "ok"]).observe(secs);
                        jwks.sort();
                        let _ = observation_tx.push((), (issuer_bytes.clone(), jwks));
                    } else {
                        OBSERVATION_SECONDS.with_label_values(&[&issuer, "err"]).observe(secs);
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

async fn fetch_jwks(open_id_config_url: &str, my_addr: Option<AccountAddress>) -> Result<Vec<JWK>> {
    let jwks_uri = fetch_jwks_uri_from_openid_config(open_id_config_url).await.map_err(|e|anyhow!("fetch_jwks failed with open-id config request: {e}"))?;
    let jwks = fetch_jwks_from_jwks_uri(my_addr, jwks_uri.as_str()).await.map_err(|e|anyhow!("fetch_jwks failed with jwks uri request: {e}"))?;
    Ok(jwks)
}
