// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//!
//! This module is to contain all networking logging information.
//!
//! ```
//! use velor_config::network_id::NetworkContext;
//! use velor_logger::info;
//! use velor_types::{PeerId, network_address::NetworkAddress};
//! use velor_network::logging::NetworkSchema;
//!
//! info!(
//!   NetworkSchema::new(&NetworkContext::mock())
//!     .remote_peer(&PeerId::random())
//!     .network_address(&NetworkAddress::mock()),
//!   field_name = "field",
//!   "Value is {} message",
//!   5
//! );
//! ```

use crate::{
    connectivity_manager::DiscoverySource,
    transport::{ConnectionId, ConnectionMetadata},
};
use velor_config::network_id::NetworkContext;
use velor_logger::Schema;
use velor_netcore::transport::ConnectionOrigin;
use velor_types::{network_address::NetworkAddress, PeerId};

#[derive(Schema)]
pub struct NetworkSchema<'a> {
    connection_id: Option<&'a ConnectionId>,
    #[schema(display)]
    connection_origin: Option<&'a ConnectionOrigin>,
    #[schema(display)]
    discovery_source: Option<&'a DiscoverySource>,
    message: Option<String>,
    #[schema(display)]
    network_address: Option<&'a NetworkAddress>,
    network_context: &'a NetworkContext,
    #[schema(display)]
    remote_peer: Option<&'a PeerId>,
}

impl<'a> NetworkSchema<'a> {
    pub fn new(network_context: &'a NetworkContext) -> Self {
        Self {
            connection_id: None,
            connection_origin: None,
            discovery_source: None,
            message: None,
            network_address: None,
            network_context,
            remote_peer: None,
        }
    }

    pub fn connection_metadata(self, metadata: &'a ConnectionMetadata) -> Self {
        self.connection_id(&metadata.connection_id)
            .connection_origin(&metadata.origin)
            .remote_peer(&metadata.remote_peer_id)
    }

    pub fn connection_metadata_with_address(self, metadata: &'a ConnectionMetadata) -> Self {
        self.connection_id(&metadata.connection_id)
            .connection_origin(&metadata.origin)
            .remote_peer(&metadata.remote_peer_id)
            .network_address(&metadata.addr)
    }
}
