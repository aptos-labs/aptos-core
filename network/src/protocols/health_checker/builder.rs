// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::storage::PeerMetadataStorage,
    protocols::health_checker::{
        interface::HealthCheckNetworkInterface, HealthChecker, HealthCheckerNetworkEvents,
        HealthCheckerNetworkSender,
    },
};
use aptos_config::network_id::NetworkContext;
use aptos_logger::prelude::*;
use aptos_time_service::TimeService;
use std::{sync::Arc, time::Duration};
use tokio::runtime::Handle;

pub struct HealthCheckerBuilder {
    service: Option<HealthChecker>,
}

impl HealthCheckerBuilder {
    pub fn new(
        network_context: NetworkContext,
        time_service: TimeService,
        ping_interval_ms: u64,
        ping_timeout_ms: u64,
        ping_failures_tolerated: u64,
        network_tx: HealthCheckerNetworkSender,
        network_rx: HealthCheckerNetworkEvents,
        peer_metadata_storage: Arc<PeerMetadataStorage>,
    ) -> Self {
        let service = HealthChecker::new(
            network_context,
            time_service,
            HealthCheckNetworkInterface::new(peer_metadata_storage, network_tx, network_rx),
            Duration::from_millis(ping_interval_ms),
            Duration::from_millis(ping_timeout_ms),
            ping_failures_tolerated,
        );
        Self {
            service: Some(service),
        }
    }

    pub fn start(&mut self, executor: &Handle) {
        if let Some(service) = self.service.take() {
            spawn_named!("[Network] HC", executor, service.start());
        }
    }
}
