// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::network_id::NetworkId;
use diem_types::PeerId;
use serde::{Deserialize, Serialize};
use short_hex_str::AsShortHexStr;
use std::fmt;

#[derive(Clone, Deserialize, Eq, Hash, PartialEq, Serialize)]
/// Identifier of a node, represented as (network_id, peer_id)
pub struct PeerNetworkId {
    network_id: NetworkId,
    peer_id: PeerId,
}

impl PeerNetworkId {
    pub fn new(network_id: NetworkId, peer_id: PeerId) -> Self {
        Self {
            network_id,
            peer_id,
        }
    }
    pub fn network_id(&self) -> NetworkId {
        self.network_id
    }

    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn random() -> Self {
        Self::new(NetworkId::Public, PeerId::random())
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn random_validator() -> Self {
        Self::new(NetworkId::Validator, PeerId::random())
    }
}

impl fmt::Debug for PeerNetworkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for PeerNetworkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.network_id(), self.peer_id().short_str(),)
    }
}
