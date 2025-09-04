// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0
use crate::config::{PeerRole, RoleType};
use velor_short_hex_str::AsShortHexStr;
use velor_types::PeerId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};
use strum_macros::EnumIter;

/// A grouping of common information between all networking code for logging.
/// This should greatly reduce the groupings between these given everywhere, and will allow
/// for logging accordingly.
#[derive(Clone, Copy, Eq, PartialEq, Serialize)]
pub struct NetworkContext {
    /// The type of node
    role: RoleType,
    #[serde(serialize_with = "NetworkId::serialize_str")]
    network_id: NetworkId,
    peer_id: PeerId,
}

impl fmt::Debug for NetworkContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for NetworkContext {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "[{},{},{}]",
            self.role,
            self.network_id.as_str(),
            self.peer_id.short_str(),
        )
    }
}

impl NetworkContext {
    pub fn new(role: RoleType, network_id: NetworkId, peer_id: PeerId) -> NetworkContext {
        NetworkContext {
            role,
            network_id,
            peer_id,
        }
    }

    pub fn role(&self) -> RoleType {
        self.role
    }

    pub fn network_id(&self) -> NetworkId {
        self.network_id
    }

    pub fn peer_id(&self) -> PeerId {
        self.peer_id
    }

    #[cfg(any(test, feature = "testing", feature = "fuzzing"))]
    pub fn mock_with_peer_id(peer_id: PeerId) -> Self {
        Self::new(RoleType::Validator, NetworkId::Validator, peer_id)
    }

    #[cfg(any(test, feature = "testing", feature = "fuzzing"))]
    pub fn mock() -> Self {
        Self::new(RoleType::Validator, NetworkId::Validator, PeerId::random())
    }
}

/// A representation of the network being used in communication.
/// There should only be one of each NetworkId used for a single node (except for NetworkId::Public),
/// and handshakes should verify that the NetworkId being used is the same during a handshake,
/// to effectively ensure communication is restricted to a network.  Network should be checked that
/// it is not the `DEFAULT_NETWORK`
#[derive(Clone, Copy, Eq, Hash, PartialEq, PartialOrd, Ord, EnumIter)]
#[repr(u8)]
pub enum NetworkId {
    Validator = 0,
    Vfn = 3,
    Public = 4,
}

// This serializer is here for backwards compatibility with the old version, once all nodes have the
// new format, we can do a migration path towards the current representations
impl Serialize for NetworkId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        #[serde(rename = "NetworkId", rename_all = "snake_case")]
        enum ConvertNetworkId {
            Validator,
            Public,
            Private(String),
        }

        let converted = match self {
            NetworkId::Validator => ConvertNetworkId::Validator,
            NetworkId::Public => ConvertNetworkId::Public,
            // TODO: Once all validators & VFNs are on this version, convert to using new serialization as number
            NetworkId::Vfn => ConvertNetworkId::Private(VFN_NETWORK.to_string()),
        };

        converted.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for NetworkId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename = "NetworkId", rename_all = "snake_case")]
        enum ConvertNetworkId {
            Validator,
            Public,
            #[allow(dead_code)]
            Private(String),
            // These are here for migration, since both need to have their representation changed
            // in the 2nd step of migration, we can move to these identifiers
            Vfn,
            NewPublic,
        }

        // A hack around NetworkId to convert the old type to the new version
        match ConvertNetworkId::deserialize(deserializer)? {
            ConvertNetworkId::Validator => Ok(NetworkId::Validator),
            ConvertNetworkId::Public => Ok(NetworkId::Public),
            ConvertNetworkId::Vfn => Ok(NetworkId::Vfn),
            ConvertNetworkId::NewPublic => Ok(NetworkId::Public),
            // Technically, there could be a different private network, but it isn't used right now
            ConvertNetworkId::Private(_) => Ok(NetworkId::Vfn),
        }
    }
}

/// Default needed to handle downstream structs that use `Default`
impl Default for NetworkId {
    fn default() -> NetworkId {
        NetworkId::Public
    }
}

impl fmt::Debug for NetworkId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Display for NetworkId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

const VFN_NETWORK: &str = "vfn";

impl NetworkId {
    pub fn is_public_network(&self) -> bool {
        self == &NetworkId::Public
    }

    pub fn is_vfn_network(&self) -> bool {
        self == &NetworkId::Vfn
    }

    pub fn is_validator_network(&self) -> bool {
        self == &NetworkId::Validator
    }

    /// Roles for a prioritization of relative upstreams
    pub fn upstream_roles(&self, role: &RoleType) -> &'static [PeerRole] {
        match self {
            NetworkId::Validator => &[PeerRole::Validator],
            NetworkId::Public => &[
                PeerRole::PreferredUpstream,
                PeerRole::Upstream,
                PeerRole::ValidatorFullNode,
            ],
            NetworkId::Vfn => match role {
                RoleType::Validator => &[],
                RoleType::FullNode => &[PeerRole::Validator],
            },
        }
    }

    /// Roles for a prioritization of relative downstreams
    pub fn downstream_roles(&self, role: &RoleType) -> &'static [PeerRole] {
        match self {
            NetworkId::Validator => &[PeerRole::Validator],
            // In order to allow fallbacks, we must allow for nodes to accept ValidatorFullNodes
            NetworkId::Public => &[
                PeerRole::ValidatorFullNode,
                PeerRole::Downstream,
                PeerRole::Known,
                PeerRole::Unknown,
            ],
            NetworkId::Vfn => match role {
                RoleType::Validator => &[PeerRole::ValidatorFullNode],
                RoleType::FullNode => &[],
            },
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            NetworkId::Validator => "Validator",
            NetworkId::Public => "Public",
            NetworkId::Vfn => VFN_NETWORK,
        }
    }

    fn serialize_str<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_str().serialize(serializer)
    }
}

impl FromStr for NetworkId {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "validator" => Ok(NetworkId::Validator),
            "public" => Ok(NetworkId::Public),
            VFN_NETWORK => Ok(NetworkId::Vfn),
            _ => Err("Invalid network name"),
        }
    }
}

#[derive(Clone, Copy, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ensure_network_id_order() {
        assert!(NetworkId::Validator < NetworkId::Vfn);
        assert!(NetworkId::Vfn < NetworkId::Public);
        assert!(NetworkId::Validator < NetworkId::Public);
    }

    #[test]
    fn test_serialization() {
        for id in [NetworkId::Validator, NetworkId::Vfn, NetworkId::Public] {
            let encoded = serde_yaml::to_string(&id).unwrap();
            let decoded: NetworkId = serde_yaml::from_str(encoded.as_str()).unwrap();
            assert_eq!(id, decoded);
            let encoded = bcs::to_bytes(&id).unwrap();
            let decoded: NetworkId = bcs::from_bytes(&encoded).unwrap();
            assert_eq!(id, decoded);
        }
    }

    #[test]
    fn test_network_context_serialization() {
        let peer_id = PeerId::random();
        let context = NetworkContext::new(RoleType::Validator, NetworkId::Vfn, peer_id);
        let expected = format!(
            "---\nrole: {}\nnetwork_id: {}\npeer_id: {:x}\n",
            RoleType::Validator,
            VFN_NETWORK,
            peer_id
        );
        assert_eq!(expected, serde_yaml::to_string(&context).unwrap());
    }

    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[serde(rename = "NetworkId", rename_all = "snake_case")]
    enum OldNetworkId {
        Validator,
        Public,
        Private(String),
    }

    #[test]
    fn test_backwards_compatibility() {
        for (old, new) in [
            (OldNetworkId::Validator, NetworkId::Validator),
            (OldNetworkId::Public, NetworkId::Public),
            (
                OldNetworkId::Private(VFN_NETWORK.to_string()),
                NetworkId::Vfn,
            ),
        ] {
            // Old version can be decoded as new version
            let encoded = serde_yaml::to_string(&old).unwrap();
            let decoded: NetworkId = serde_yaml::from_str(&encoded).unwrap();
            assert_eq!(new, decoded);
            let encoded = bcs::to_bytes(&old).unwrap();
            let decoded: NetworkId = bcs::from_bytes(&encoded).unwrap();
            assert_eq!(new, decoded);

            // New version can be decoded as old version
            let encoded = serde_yaml::to_string(&new).unwrap();
            let decoded: OldNetworkId = serde_yaml::from_str(&encoded).unwrap();
            assert_eq!(old, decoded);
            let encoded = bcs::to_bytes(&new).unwrap();
            let decoded: OldNetworkId = bcs::from_bytes(&encoded).unwrap();
            assert_eq!(old, decoded);
        }
    }
}
