// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    application::{interface::PeerStateChange, storage::PeerMetadataStorage},
    transport::ConnectionMetadata,
};
use std::sync::Arc;

/// An interface around convenience for connection based storage
pub trait ConnectionStorage {
    /// Insert a new connection
    fn insert_connection(&self, connection_metadata: ConnectionMetadata);

    /// Remove a connection based on the previous connection's metadata
    fn remove_connection(&self, connection_metadata: ConnectionMetadata);
}

/// Simple implementation of `PeerManagementInterface` for `PeerManager`
pub struct PeerMetadataManagement {
    peer_metadata_storage: Arc<PeerMetadataStorage>,
}

impl PeerMetadataManagement {
    pub fn new(peer_metadata_storage: Arc<PeerMetadataStorage>) -> Self {
        PeerMetadataManagement {
            peer_metadata_storage,
        }
    }
}

impl ConnectionStorage for PeerMetadataManagement {
    fn insert_connection(&self, connection_metadata: ConnectionMetadata) {
        self.peer_metadata_storage
            .insert_connection(connection_metadata)
    }

    fn remove_connection(&self, connection_metadata: ConnectionMetadata) {
        self.peer_metadata_storage
            .remove_connection(connection_metadata)
    }
}

impl PeerStateChange for PeerMetadataManagement {
    fn peer_metadata_storage(&self) -> &PeerMetadataStorage {
        &self.peer_metadata_storage
    }
}
