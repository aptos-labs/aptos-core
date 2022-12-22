// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! Network stresser
//!
//! NetPerf is used to stress the network laayer to gouge potential performance capabilities and ease
//! network realted performance profiling and debugging
//!

use crate::application::storage::PeerMetadataStorage;
use aptos_config::network_id::{NetworkContext, PeerNetworkId};
use std::sync::Arc;

pub struct NetPerf {
    network_context: NetworkContext,
    peers: Arc<PeerMetadataStorage>,
}

impl NetPerf {
    pub fn new(
        network_context: NetworkContext,
        peers: std::sync::Arc<PeerMetadataStorage>,
    ) -> Self {
        NetPerf {
            network_context,
            peers,
        }
    }
    pub async fn start(mut self) {}
}
