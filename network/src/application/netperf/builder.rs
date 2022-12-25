// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::netperf::{
        interface::{NetPerfNetworkEvents, NetPerfNetworkSender},
        NetPerf,
    },
    application::storage::PeerMetadataStorage,
};
use aptos_config::network_id::NetworkContext;
use aptos_logger::prelude::*;
use std::sync::Arc;
use tokio::runtime::Handle;

pub struct NetPerfBuilder {
    service: Option<NetPerf>,
}

impl NetPerfBuilder {
    pub fn new(
        network_context: NetworkContext,
        peer_metadata_storage: Arc<PeerMetadataStorage>,
        network_tx: Arc<NetPerfNetworkSender>,
        network_rx: NetPerfNetworkEvents,
        netperf_port: u16,
    ) -> Self {
        let service = NetPerf::new(
            network_context,
            peer_metadata_storage,
            network_tx,
            network_rx,
            netperf_port,
        );
        Self {
            service: Some(service),
        }
    }

    pub fn start(&mut self, executor: &Handle) {
        if let Some(service) = self.service.take() {
            spawn_named!("[Network] NetPerf", executor, service.start());
        }
    }
}
