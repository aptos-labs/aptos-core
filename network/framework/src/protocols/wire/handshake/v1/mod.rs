// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module defines the structs transported during the network handshake protocol v1.
//! These should serialize as per the [VelorNet Handshake v1 Specification].
//!
//! During the v1 Handshake protocol, both end-points of a connection send a serialized and
//! length-prefixed [`HandshakeMsg`] to each other. The handshake message contains a map from
//! supported messaging protocol versions to a bit vector representing application protocols
//! supported over that messaging protocol. On receipt, both ends will determine the highest
//! intersecting messaging protocol version and use that for the remainder of the session.
//!
//! [VelorNet Handshake v1 Specification]: https://github.com/velor-chain/velor-core/blob/main/specifications/network/handshake-v1.md

use crate::counters::{start_serialization_timer, DESERIALIZATION_LABEL, SERIALIZATION_LABEL};
use anyhow::anyhow;
use velor_compression::client::CompressionClient;
use velor_config::{config::MAX_APPLICATION_MESSAGE_SIZE, network_id::NetworkId};
use velor_types::chain_id::ChainId;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt,
    iter::{FromIterator, Iterator},
    ops::{BitAnd, BitOr},
};
use thiserror::Error;

#[cfg(test)]
mod test;

//
// ProtocolId
//

pub const USER_INPUT_RECURSION_LIMIT: usize = 32;
pub const RECURSION_LIMIT: usize = 64;

/// Unique identifier associated with each application protocol.
#[repr(u8)]
#[derive(Clone, Copy, Hash, Eq, PartialEq, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum ProtocolId {
    ConsensusRpcBcs = 0,
    ConsensusDirectSendBcs = 1,
    MempoolDirectSend = 2,
    StateSyncDirectSend = 3,
    DiscoveryDirectSend = 4, // Currently unused
    HealthCheckerRpc = 5,
    ConsensusDirectSendJson = 6, // Json provides flexibility for backwards compatible upgrade
    ConsensusRpcJson = 7,
    StorageServiceRpc = 8,
    MempoolRpc = 9, // Currently unused
    PeerMonitoringServiceRpc = 10,
    ConsensusRpcCompressed = 11,
    ConsensusDirectSendCompressed = 12,
    NetbenchDirectSend = 13,
    NetbenchRpc = 14,
    DKGDirectSendCompressed = 15,
    DKGDirectSendBcs = 16,
    DKGDirectSendJson = 17,
    DKGRpcCompressed = 18,
    DKGRpcBcs = 19,
    DKGRpcJson = 20,
    JWKConsensusDirectSendCompressed = 21,
    JWKConsensusDirectSendBcs = 22,
    JWKConsensusDirectSendJson = 23,
    JWKConsensusRpcCompressed = 24,
    JWKConsensusRpcBcs = 25,
    JWKConsensusRpcJson = 26,
    ConsensusObserver = 27,
    ConsensusObserverRpc = 28,
}

/// The encoding types for Protocols
enum Encoding {
    Bcs(usize),
    CompressedBcs(usize),
    Json,
}

impl ProtocolId {
    pub fn as_str(self) -> &'static str {
        use ProtocolId::*;
        match self {
            ConsensusRpcBcs => "ConsensusRpcBcs",
            ConsensusDirectSendBcs => "ConsensusDirectSendBcs",
            MempoolDirectSend => "MempoolDirectSend",
            StateSyncDirectSend => "StateSyncDirectSend",
            DiscoveryDirectSend => "DiscoveryDirectSend",
            HealthCheckerRpc => "HealthCheckerRpc",
            ConsensusDirectSendJson => "ConsensusDirectSendJson",
            ConsensusRpcJson => "ConsensusRpcJson",
            StorageServiceRpc => "StorageServiceRpc",
            MempoolRpc => "MempoolRpc",
            PeerMonitoringServiceRpc => "PeerMonitoringServiceRpc",
            ConsensusRpcCompressed => "ConsensusRpcCompressed",
            ConsensusDirectSendCompressed => "ConsensusDirectSendCompressed",
            NetbenchDirectSend => "NetbenchDirectSend",
            NetbenchRpc => "NetbenchRpc",
            DKGDirectSendCompressed => "DKGDirectSendCompressed",
            DKGDirectSendBcs => "DKGDirectSendBcs",
            DKGDirectSendJson => "DKGDirectSendJson",
            DKGRpcCompressed => "DKGRpcCompressed",
            DKGRpcBcs => "DKGRpcBcs",
            DKGRpcJson => "DKGRpcJson",
            JWKConsensusDirectSendCompressed => "JWKConsensusDirectSendCompressed",
            JWKConsensusDirectSendBcs => "JWKConsensusDirectSendBcs",
            JWKConsensusDirectSendJson => "JWKConsensusDirectSendJson",
            JWKConsensusRpcCompressed => "JWKConsensusRpcCompressed",
            JWKConsensusRpcBcs => "JWKConsensusRpcBcs",
            JWKConsensusRpcJson => "JWKConsensusRpcJson",
            ConsensusObserver => "ConsensusObserver",
            ConsensusObserverRpc => "ConsensusObserverRpc",
        }
    }

    /// Returns all protocol ID types
    pub fn all() -> &'static [ProtocolId] {
        &[
            ProtocolId::ConsensusRpcBcs,
            ProtocolId::ConsensusDirectSendBcs,
            ProtocolId::MempoolDirectSend,
            ProtocolId::StateSyncDirectSend,
            ProtocolId::DiscoveryDirectSend,
            ProtocolId::HealthCheckerRpc,
            ProtocolId::ConsensusDirectSendJson,
            ProtocolId::ConsensusRpcJson,
            ProtocolId::StorageServiceRpc,
            ProtocolId::MempoolRpc,
            ProtocolId::PeerMonitoringServiceRpc,
            ProtocolId::ConsensusRpcCompressed,
            ProtocolId::ConsensusDirectSendCompressed,
            ProtocolId::NetbenchDirectSend,
            ProtocolId::NetbenchRpc,
            ProtocolId::DKGDirectSendCompressed,
            ProtocolId::DKGDirectSendBcs,
            ProtocolId::DKGDirectSendJson,
            ProtocolId::DKGRpcCompressed,
            ProtocolId::DKGRpcBcs,
            ProtocolId::DKGRpcJson,
            ProtocolId::JWKConsensusDirectSendCompressed,
            ProtocolId::JWKConsensusDirectSendBcs,
            ProtocolId::JWKConsensusDirectSendJson,
            ProtocolId::JWKConsensusRpcCompressed,
            ProtocolId::JWKConsensusRpcBcs,
            ProtocolId::JWKConsensusRpcJson,
            ProtocolId::ConsensusObserver,
            ProtocolId::ConsensusObserverRpc,
        ]
    }

    /// Specifies how to encode messages for a given `ProtocolId`
    fn encoding(self) -> Encoding {
        match self {
            ProtocolId::ConsensusDirectSendJson | ProtocolId::ConsensusRpcJson => Encoding::Json,
            ProtocolId::ConsensusDirectSendCompressed | ProtocolId::ConsensusRpcCompressed => {
                Encoding::CompressedBcs(RECURSION_LIMIT)
            },
            ProtocolId::ConsensusObserver => Encoding::CompressedBcs(RECURSION_LIMIT),
            ProtocolId::DKGDirectSendCompressed | ProtocolId::DKGRpcCompressed => {
                Encoding::CompressedBcs(RECURSION_LIMIT)
            },
            ProtocolId::JWKConsensusDirectSendCompressed
            | ProtocolId::JWKConsensusRpcCompressed => Encoding::CompressedBcs(RECURSION_LIMIT),
            ProtocolId::MempoolDirectSend => Encoding::CompressedBcs(USER_INPUT_RECURSION_LIMIT),
            ProtocolId::MempoolRpc => Encoding::Bcs(USER_INPUT_RECURSION_LIMIT),
            _ => Encoding::Bcs(RECURSION_LIMIT),
        }
    }

    /// Returns the compression client label based on the current protocol id
    fn get_compression_client(self) -> CompressionClient {
        match self {
            ProtocolId::ConsensusDirectSendCompressed | ProtocolId::ConsensusRpcCompressed => {
                CompressionClient::Consensus
            },
            ProtocolId::ConsensusObserver => CompressionClient::ConsensusObserver,
            ProtocolId::MempoolDirectSend => CompressionClient::Mempool,
            ProtocolId::DKGDirectSendCompressed | ProtocolId::DKGRpcCompressed => {
                CompressionClient::DKG
            },
            ProtocolId::JWKConsensusDirectSendCompressed
            | ProtocolId::JWKConsensusRpcCompressed => CompressionClient::JWKConsensus,
            protocol_id => unreachable!(
                "The given protocol ({:?}) should not be using compression!",
                protocol_id
            ),
        }
    }

    /// Serializes the given message into bytes (based on the protocol ID
    /// and encoding to use).
    pub fn to_bytes<T: Serialize>(&self, value: &T) -> anyhow::Result<Vec<u8>> {
        // Start the serialization timer
        let serialization_timer = start_serialization_timer(*self, SERIALIZATION_LABEL);

        // Serialize the message
        let result = match self.encoding() {
            Encoding::Bcs(limit) => self.bcs_encode(value, limit),
            Encoding::CompressedBcs(limit) => {
                let compression_client = self.get_compression_client();
                let bcs_bytes = self.bcs_encode(value, limit)?;
                velor_compression::compress(
                    bcs_bytes,
                    compression_client,
                    MAX_APPLICATION_MESSAGE_SIZE,
                )
                .map_err(|e| anyhow!("{:?}", e))
            },
            Encoding::Json => serde_json::to_vec(value).map_err(|e| anyhow!("{:?}", e)),
        };

        // Only record the duration if serialization was successful
        if result.is_ok() {
            serialization_timer.observe_duration();
        }

        result
    }

    /// Deserializes the given bytes into a typed message (based on the
    /// protocol ID and encoding to use).
    pub fn from_bytes<T: DeserializeOwned>(&self, bytes: &[u8]) -> anyhow::Result<T> {
        // Start the deserialization timer
        let deserialization_timer = start_serialization_timer(*self, DESERIALIZATION_LABEL);

        // Deserialize the message
        let result = match self.encoding() {
            Encoding::Bcs(limit) => self.bcs_decode(bytes, limit),
            Encoding::CompressedBcs(limit) => {
                let compression_client = self.get_compression_client();
                let raw_bytes = velor_compression::decompress(
                    &bytes.to_vec(),
                    compression_client,
                    MAX_APPLICATION_MESSAGE_SIZE,
                )
                .map_err(|e| anyhow! {"{:?}", e})?;
                self.bcs_decode(&raw_bytes, limit)
            },
            Encoding::Json => serde_json::from_slice(bytes).map_err(|e| anyhow!("{:?}", e)),
        };

        // Only record the duration if deserialization was successful
        if result.is_ok() {
            deserialization_timer.observe_duration();
        }

        result
    }

    /// Serializes the value using BCS encoding (with a specified limit)
    fn bcs_encode<T: Serialize>(&self, value: &T, limit: usize) -> anyhow::Result<Vec<u8>> {
        bcs::to_bytes_with_limit(value, limit).map_err(|e| anyhow!("{:?}", e))
    }

    /// Deserializes the value using BCS encoding (with a specified limit)
    fn bcs_decode<T: DeserializeOwned>(&self, bytes: &[u8], limit: usize) -> anyhow::Result<T> {
        bcs::from_bytes_with_limit(bytes, limit).map_err(|e| anyhow!("{:?}", e))
    }
}

impl fmt::Debug for ProtocolId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for ProtocolId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

//
// ProtocolIdSet
//

/// A compact representation for a set of [`ProtocolId`]s. Internally, this is a
/// bitvec which supports at most 256 bits.
///
/// These sets are sent over-the-wire in the initial [`HandshakeMsg`] to other
/// VelorNet peers in order to negotiate the set of common supported protocols for
/// use on a new VelorNet connection.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct ProtocolIdSet(velor_bitvec::BitVec);

impl ProtocolIdSet {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn all_known() -> Self {
        Self::from_iter(ProtocolId::all())
    }

    #[cfg(test)]
    pub fn mock() -> Self {
        Self::from_iter([ProtocolId::DiscoveryDirectSend])
    }

    pub fn is_empty(&self) -> bool {
        self.0.all_zeros()
    }

    /// Iterate over all `ProtocolId`s, ignoring any that our node version
    /// doesn't understand or doesn't yet support.
    pub fn iter(&self) -> impl Iterator<Item = ProtocolId> + '_ {
        self.0
            .iter_ones()
            .filter_map(|idx| bcs::from_bytes(&[idx as u8]).ok())
    }

    /// Find the intersection between two sets of protocols.
    pub fn intersect(&self, other: &ProtocolIdSet) -> ProtocolIdSet {
        ProtocolIdSet(self.0.bitand(&other.0))
    }

    /// Return the union of two sets of protocols.
    pub fn union(&self, other: &ProtocolIdSet) -> ProtocolIdSet {
        ProtocolIdSet(self.0.bitor(&other.0))
    }

    /// Returns if the protocol is set.
    pub fn contains(&self, protocol: ProtocolId) -> bool {
        self.0.is_set(protocol as u16)
    }

    /// Insert a new protocol into the set.
    pub fn insert(&mut self, protocol: ProtocolId) {
        self.0.set(protocol as u16)
    }
}

impl FromIterator<ProtocolId> for ProtocolIdSet {
    fn from_iter<T: IntoIterator<Item = ProtocolId>>(iter: T) -> Self {
        Self(iter.into_iter().map(|protocol| protocol as u8).collect())
    }
}

impl<'a> FromIterator<&'a ProtocolId> for ProtocolIdSet {
    fn from_iter<T: IntoIterator<Item = &'a ProtocolId>>(iter: T) -> Self {
        iter.into_iter().copied().collect()
    }
}

//
// MessageProtocolVersion
//

/// Enum representing different versions of the Velor network protocol. These
/// should be listed from old to new, old having the smallest value.  We derive
/// [`PartialOrd`] since nodes need to find highest intersecting protocol version.
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy, Hash, Deserialize, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub enum MessagingProtocolVersion {
    V1 = 0,
}

impl MessagingProtocolVersion {
    fn as_str(&self) -> &str {
        match self {
            Self::V1 => "V1",
        }
    }
}

impl fmt::Debug for MessagingProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for MessagingProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

//
// HandshakeMsg
//

/// An enum to list the possible errors during the Atpos handshake negotiation
#[derive(Debug, Error, Eq, PartialEq)]
pub enum HandshakeError {
    #[error("velor-handshake: the received message has a different chain id: {0}, expected: {1}")]
    InvalidChainId(ChainId, ChainId),
    #[error(
        "velor-handshake: the received message has an different network id: {0}, expected: {1}"
    )]
    InvalidNetworkId(NetworkId, NetworkId),
    #[error("velor-handshake: could not find an intersection of supported protocol with the peer")]
    NoCommonProtocols,
}

/// The HandshakeMsg contains a mapping from [`MessagingProtocolVersion`]
/// suppported by the node to a bit-vector specifying application-level protocols
/// supported over that version.
#[derive(Clone, Deserialize, Serialize, Default)]
pub struct HandshakeMsg {
    pub supported_protocols: BTreeMap<MessagingProtocolVersion, ProtocolIdSet>,
    pub chain_id: ChainId,
    pub network_id: NetworkId,
}

impl HandshakeMsg {
    /// Useful function for tests
    #[cfg(test)]
    pub fn new_for_testing() -> Self {
        Self::from_supported([ProtocolId::HealthCheckerRpc].iter().collect())
    }

    #[cfg(test)]
    pub fn from_supported(protos: ProtocolIdSet) -> Self {
        let mut supported_protocols = BTreeMap::new();
        supported_protocols.insert(MessagingProtocolVersion::V1, protos);
        Self {
            chain_id: ChainId::test(),
            network_id: NetworkId::Validator,
            supported_protocols,
        }
    }

    /// This function:
    /// 1. verifies that both HandshakeMsg are compatible and
    /// 2. finds out the intersection of protocols that is supported
    pub fn perform_handshake(
        &self,
        other: &HandshakeMsg,
    ) -> Result<(MessagingProtocolVersion, ProtocolIdSet), HandshakeError> {
        // verify that both peers are on the same chain
        if self.chain_id != other.chain_id {
            return Err(HandshakeError::InvalidChainId(
                other.chain_id,
                self.chain_id,
            ));
        }

        // verify that both peers are on the same network
        if self.network_id != other.network_id {
            return Err(HandshakeError::InvalidNetworkId(
                other.network_id,
                self.network_id,
            ));
        }

        // find the greatest common MessagingProtocolVersion where we both support
        // at least one common ProtocolId.
        for (our_handshake_version, our_protocols) in self.supported_protocols.iter().rev() {
            if let Some(their_protocols) = other.supported_protocols.get(our_handshake_version) {
                let common_protocols = our_protocols.intersect(their_protocols);

                if !common_protocols.is_empty() {
                    return Ok((*our_handshake_version, common_protocols));
                }
            }
        }

        // no intersection found
        Err(HandshakeError::NoCommonProtocols)
    }
}

impl fmt::Debug for HandshakeMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for HandshakeMsg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{},{},{:?}]",
            self.chain_id, self.network_id, self.supported_protocols
        )
    }
}
