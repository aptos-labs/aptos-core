// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::counters::OBSERVATION_SECONDS;
use anyhow::{anyhow, Result};
use api_types::relayer::GLOBAL_RELAYER;
use aptos_channels::aptos_channel;
use aptos_jwk_utils::{fetch_jwks_from_jwks_uri, fetch_jwks_uri_from_openid_config};
use aptos_logger::{debug, error, info};
use aptos_types::jwks::{jwk::JWK, unsupported::UnsupportedJWK, Issuer};
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
        observation_tx: aptos_channel::Sender<(), (Issuer, Vec<JWK>)>,
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

        if issuer.starts_with("gravity://") {
            let relayer = GLOBAL_RELAYER.get().unwrap();
            let r = relayer
                .add_uri(issuer.as_str(), open_id_config_url.as_str())
                .await;
            if r.is_err() {
                error!(
                    "Failed to add issuer to relayer with uri={:?}, error={:?}",
                    open_id_config_url, r.unwrap_err(),
                );
                return;
            }
        }

        loop {
            tokio::select! {
                _ = interval.tick().fuse() => {
                    let timer = Instant::now();
                    let result = fetch_jwks(open_id_config_url.as_str(), my_addr, issuer.as_str()).await;
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

async fn fetch_jwks_with_relayer(issuer: &str) -> Result<Vec<JWK>> {
    let relayer = GLOBAL_RELAYER.get().unwrap();
    let last_state = relayer.get_last_state(issuer).await.unwrap();
    let jwks = JWK::Unsupported(UnsupportedJWK {
        id: issuer.as_bytes().to_vec(),
        payload: last_state,
    });
    Ok(vec![jwks])
}

async fn fetch_jwks(open_id_config_url: &str, my_addr: Option<AccountAddress>, issuer: &str) -> Result<Vec<JWK>> {
    if issuer.starts_with("gravity://") {
        return fetch_jwks_with_relayer(issuer).await;
    }
    let jwks_uri = fetch_jwks_uri_from_openid_config(open_id_config_url)
        .await
        .map_err(|e| anyhow!("fetch_jwks failed with open-id config request: {e}"))?;
    let jwks = fetch_jwks_from_jwks_uri(my_addr, jwks_uri.as_str())
        .await
        .map_err(|e| anyhow!("fetch_jwks failed with jwks uri request: {e}"))?;
    Ok(jwks)
}
