// Copyright (c) The Diem Core Contributors
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
fn protocols_to_from_vec() {
    let supported_protocols: SupportedProtocols =
        [ProtocolId::ConsensusRpc, ProtocolId::MempoolDirectSend]
            .iter()
            .into();
    assert_eq!(
        SupportedProtocols::from(
            (supported_protocols.clone().try_into() as Result<Vec<ProtocolId>, _>)
                .unwrap()
                .iter()
        ),
        supported_protocols
    );
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
        [ProtocolId::ConsensusRpc, ProtocolId::DiscoveryDirectSend]
            .iter()
            .into(),
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
        [ProtocolId::ConsensusRpc, ProtocolId::MempoolDirectSend]
            .iter()
            .into(),
    );
    let h2 = HandshakeMsg {
        chain_id,
        network_id,
        supported_protocols,
    };

    assert_eq!(
        (
            MessagingProtocolVersion::V1,
            [ProtocolId::ConsensusRpc].iter().into()
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
    supported_protocols.insert(MessagingProtocolVersion::V1, SupportedProtocols::default());
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
    assert!(SupportedProtocols::default().is_empty());
    assert!(SupportedProtocols::from(ProtocolId::all().iter())
        .intersect(&SupportedProtocols::default())
        .is_empty());
    assert!(!SupportedProtocols::from(ProtocolId::all().iter()).is_empty());
}

// Ensure we can handshake with a peer advertising some totally unknown ProtocoId's.

#[test]
fn ignore_unknown_protocols() {
    let all_known_protos = HandshakeMsg::from_supported(SupportedProtocols::from(
        [
            ProtocolId::MempoolDirectSend,
            ProtocolId::StateSyncDirectSend,
        ]
        .iter(),
    ));

    let some_unknown_protos =
        HandshakeMsg::from_supported(SupportedProtocols(bitvec::BitVec::from_iter([
            ProtocolId::MempoolDirectSend as u8,
            66,
            234,
        ])));

    let all_unknown_protos =
        HandshakeMsg::from_supported(SupportedProtocols(bitvec::BitVec::from_iter([42, 99, 123])));

    // Case 1: the other set contains some unknown protocols, but we can still
    // find a common protocol.

    let (_, common_protos) = all_known_protos
        .perform_handshake(&some_unknown_protos)
        .unwrap();
    assert_eq!(
        common_protos,
        SupportedProtocols::from([ProtocolId::MempoolDirectSend].iter())
    );

    // Case 2: the other set contains exclusively unknown protocols and so we
    // can't communicate.

    assert_eq!(
        all_known_protos
            .perform_handshake(&all_unknown_protos)
            .unwrap_err(),
        HandshakeError::NoCommonProtocols,
    );
}
