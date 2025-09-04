// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::storage::PeersAndMetadata,
    connectivity_manager::{ConnectivityManager, ConnectivityRequest},
    counters,
    peer_manager::{conn_notifs_channel, ConnectionRequestSender},
};
use velor_config::{config::PeerSet, network_id::NetworkContext};
use velor_time_service::TimeService;
use std::{sync::Arc, time::Duration};
use tokio::runtime::Handle;
use tokio_retry::strategy::ExponentialBackoff;

pub type ConnectivityManagerService = ConnectivityManager<ExponentialBackoff>;

pub struct ConnectivityManagerBuilder {
    connectivity_manager: Option<ConnectivityManagerService>,
    conn_mgr_reqs_tx: velor_channels::Sender<ConnectivityRequest>,
}

impl ConnectivityManagerBuilder {
    pub fn create(
        network_context: NetworkContext,
        time_service: TimeService,
        peers_and_metadata: Arc<PeersAndMetadata>,
        seeds: PeerSet,
        connectivity_check_interval_ms: u64,
        backoff_base: u64,
        max_connection_delay_ms: u64,
        channel_size: usize,
        connection_reqs_tx: ConnectionRequestSender,
        connection_notifs_rx: conn_notifs_channel::Receiver,
        outbound_connection_limit: Option<usize>,
        mutual_authentication: bool,
        enable_latency_aware_dialing: bool,
    ) -> Self {
        let (conn_mgr_reqs_tx, conn_mgr_reqs_rx) = velor_channels::new(
            channel_size,
            &counters::PENDING_CONNECTIVITY_MANAGER_REQUESTS,
        );

        Self {
            conn_mgr_reqs_tx,
            connectivity_manager: Some(ConnectivityManager::new(
                network_context,
                time_service,
                peers_and_metadata,
                seeds,
                connection_reqs_tx,
                connection_notifs_rx,
                conn_mgr_reqs_rx,
                Duration::from_millis(connectivity_check_interval_ms),
                ExponentialBackoff::from_millis(backoff_base).factor(1000),
                Duration::from_millis(max_connection_delay_ms),
                outbound_connection_limit,
                mutual_authentication,
                enable_latency_aware_dialing,
            )),
        }
    }

    pub fn conn_mgr_reqs_tx(&self) -> velor_channels::Sender<ConnectivityRequest> {
        self.conn_mgr_reqs_tx.clone()
    }

    pub fn start(&mut self, executor: &Handle) {
        let conn_mgr = self
            .connectivity_manager
            .take()
            .expect("Service Must be present");
        executor.spawn(conn_mgr.start());
    }
}
