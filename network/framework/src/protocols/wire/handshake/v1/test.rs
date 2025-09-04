// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use super::*;
use std::iter::FromIterator;

// Ensure serialization of MessagingProtocolVersion enum takes 1 byte.
#[test]
fn net_protocol() -> bcs::Result<()> {
    let protocol = MessagingProtocolVersion::V1;
    assert_eq!(bcs::to_bytes(&protocol)?, vec![0x00]);
    Ok(())
}

#[test]
fn protocols_to_from_iter() {
    let supported_protocols: ProtocolIdSet =
        ProtocolIdSet::from_iter([ProtocolId::ConsensusRpcBcs, ProtocolId::MempoolDirectSend]);
    assert_eq!(
        ProtocolIdSet::from_iter(supported_protocols.iter()),
        supported_protocols,
    );
}

#[test]
fn test_as_u8_serde_equiv() {
    for protocol in ProtocolId::all() {
        let protocol_as_u8_repr = *protocol as u8;
        let protocol_bcs_repr = bcs::to_bytes(protocol).unwrap();
        assert_eq!(protocol_bcs_repr, vec![protocol_as_u8_repr]);
        assert_eq!(
            bcs::from_bytes::<ProtocolId>(&[protocol_as_u8_repr]).unwrap(),
            *protocol,
        );
    }
}

#[test]
fn represents_same_network() {
    let mut handshake_msg = HandshakeMsg::new_for_testing();
    handshake_msg.network_id = NetworkId::Vfn;

    // succeeds: Positive case
    let h1 = handshake_msg.clone();
    let h2 = handshake_msg.clone();
    h1.perform_handshake(&h2).unwrap();

    // fails: different network
    let mut h2 = handshake_msg.clone();
    h2.network_id = NetworkId::Public;
    h1.perform_handshake(&h2).unwrap_err();

    // fails: different chain
    let mut h2 = handshake_msg;
    h2.chain_id = ChainId::new(h1.chain_id.id() + 1);
    h1.perform_handshake(&h2).unwrap_err();
}

#[test]
fn common_protocols() {
    let network_id = NetworkId::default();
    let chain_id = ChainId::default();
    let mut supported_protocols = BTreeMap::new();
    supported_protocols.insert(
        MessagingProtocolVersion::V1,
        ProtocolIdSet::from_iter([ProtocolId::ConsensusRpcBcs, ProtocolId::DiscoveryDirectSend]),
    );

    let h1 = HandshakeMsg {
        chain_id,
        network_id,
        supported_protocols,
    };

    // Case 1: One intersecting protocol is found for common messaging protocol version.
    let mut supported_protocols = BTreeMap::new();
    supported_protocols.insert(
        MessagingProtocolVersion::V1,
        ProtocolIdSet::from_iter([ProtocolId::ConsensusRpcBcs, ProtocolId::MempoolDirectSend]),
    );
    let h2 = HandshakeMsg {
        chain_id,
        network_id,
        supported_protocols,
    };

    assert_eq!(
        (
            MessagingProtocolVersion::V1,
            ProtocolIdSet::from_iter([ProtocolId::ConsensusRpcBcs]),
        ),
        h1.perform_handshake(&h2).unwrap()
    );

    // Case 2: No intersecting messaging protocol version.
    let h2 = HandshakeMsg {
        chain_id,
        network_id,
        supported_protocols: BTreeMap::new(),
    };
    assert_eq!(
        h1.perform_handshake(&h2).unwrap_err(),
        HandshakeError::NoCommonProtocols,
    );

    // Case 3: Intersecting messaging protocol version is present, but no intersecting protocols.
    let mut supported_protocols = BTreeMap::new();
    supported_protocols.insert(MessagingProtocolVersion::V1, ProtocolIdSet::empty());
    let h2 = HandshakeMsg {
        supported_protocols,
        chain_id,
        network_id,
    };
    assert_eq!(
        h1.perform_handshake(&h2).unwrap_err(),
        HandshakeError::NoCommonProtocols,
    );
}

#[test]
fn is_empty() {
    assert!(ProtocolIdSet::empty().is_empty());
    assert!(ProtocolIdSet::all_known()
        .intersect(&ProtocolIdSet::empty())
        .is_empty());
    assert!(ProtocolIdSet::empty()
        .intersect(&ProtocolIdSet::all_known())
        .is_empty());
    assert_eq!(
        ProtocolIdSet::all_known().union(&ProtocolIdSet::empty()),
        ProtocolIdSet::all_known()
    );
    assert_eq!(
        ProtocolIdSet::empty().union(&ProtocolIdSet::all_known()),
        ProtocolIdSet::all_known()
    );
    assert!(!ProtocolIdSet::all_known().is_empty());
}

// Ensure we can handshake with a peer advertising some totally unknown ProtocoId's.

#[test]
fn ignore_unknown_protocols() {
    let all_known_protos = ProtocolIdSet::from_iter([
        ProtocolId::MempoolDirectSend,
        ProtocolId::StateSyncDirectSend,
    ]);
    let all_known_hs = HandshakeMsg::from_supported(all_known_protos);

    let some_unknown_protos = ProtocolIdSet(velor_bitvec::BitVec::from_iter([
        ProtocolId::MempoolDirectSend as u8,
        66,
        234,
    ]));
    let some_unknown_hs = HandshakeMsg::from_supported(some_unknown_protos.clone());

    let all_unknown_protos = ProtocolIdSet(velor_bitvec::BitVec::from_iter([42, 99, 123]));
    let all_unknown_hs = HandshakeMsg::from_supported(all_unknown_protos.clone());

    // Case 1: the other set contains some unknown protocols, but we can still
    // find a common protocol.

    let (_, common_protos) = all_known_hs.perform_handshake(&some_unknown_hs).unwrap();
    assert_eq!(
        common_protos,
        ProtocolIdSet::from_iter([ProtocolId::MempoolDirectSend])
    );
    assert_eq!(
        ProtocolIdSet::from_iter(some_unknown_protos.iter()),
        ProtocolIdSet::from_iter([ProtocolId::MempoolDirectSend]),
    );

    // Case 2: the other set contains exclusively unknown protocols and so we
    // can't communicate.

    assert_eq!(
        all_known_hs.perform_handshake(&all_unknown_hs).unwrap_err(),
        HandshakeError::NoCommonProtocols,
    );
    assert_eq!(
        ProtocolIdSet::from_iter(all_unknown_protos.iter()),
        ProtocolIdSet::empty(),
    );
}
