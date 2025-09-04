// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::counters::OBSERVATION_SECONDS;
use anyhow::{anyhow, Result};
use velor_channels::velor_channel;
use velor_jwk_utils::{fetch_jwks_from_jwks_uri, fetch_jwks_uri_from_openid_config};
use velor_logger::{debug, info};
use velor_types::jwks::{jwk::JWK, Issuer};
use futures::{FutureExt, StreamExt};
use move_core_types::account_address::AccountAddress;
use std::time::{Duration, Instant};
use tokio::{sync::oneshot, task::JoinHandle, time::MissedTickBehavior};

/// A process thread that periodically fetch JWKs of a provider and push it back to JWKManager.
pub struct JWKObserver {
    close_tx: oneshot::Sender<()>,
    join_handle: JoinHandle<()>,
}

impl JWKObserver {
    pub fn spawn(
        epoch: u64,
        my_addr: AccountAddress,
        issuer: String,
        config_url: String,
        fetch_interval: Duration,
        observation_tx: velor_channel::Sender<(), (Issuer, Vec<JWK>)>,
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
            issuer = issuer,
            config_url = config_url,
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
        issuer: String,
        open_id_config_url: String,
        observation_tx: velor_channel::Sender<(), (Issuer, Vec<JWK>)>,
        close_rx: oneshot::Receiver<()>,
    ) {
        let mut interval = tokio::time::interval(fetch_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let mut close_rx = close_rx.into_stream();
        let my_addr = if cfg!(feature = "smoke-test") {
            // Include self validator address in JWK request,
            // so dummy OIDC providers in smoke tests can do things like "key A for validator 1, key B for validator 2".
            Some(my_addr)
        } else {
            None
        };

        loop {
            tokio::select! {
                _ = interval.tick().fuse() => {
                    let timer = Instant::now();
                    let result = fetch_jwks(open_id_config_url.as_str(), my_addr).await;
                    debug!(issuer = issuer, "observe_result={:?}", result);
                    let secs = timer.elapsed().as_secs_f64();
                    if let Ok(mut jwks) = result {
                        OBSERVATION_SECONDS.with_label_values(&[issuer.as_str(), "ok"]).observe(secs);
                        jwks.sort();
                        let _ = observation_tx.push((), (issuer.as_bytes().to_vec(), jwks));
                    } else {
                        OBSERVATION_SECONDS.with_label_values(&[issuer.as_str(), "err"]).observe(secs);
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
    let jwks_uri = fetch_jwks_uri_from_openid_config(open_id_config_url)
        .await
        .map_err(|e| anyhow!("fetch_jwks failed with open-id config request: {e}"))?;
    let jwks = fetch_jwks_from_jwks_uri(my_addr, jwks_uri.as_str())
        .await
        .map_err(|e| anyhow!("fetch_jwks failed with jwks uri request: {e}"))?;
    Ok(jwks)
}
