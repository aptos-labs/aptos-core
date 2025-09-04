// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Integration tests for validator_network.
use crate::dummy::{setup_network, DummyMsg};
use velor_network::{application::interface::NetworkClientInterface, protocols::network::Event};
use futures::{future::join, StreamExt};
use std::time::Duration;

#[test]
fn test_network_builder() {
    setup_network();
}

#[test]
fn test_direct_send() {
    ::velor_logger::Logger::init_for_testing();
    let tn = setup_network();
    let dialer_peer = tn.dialer_peer;
    let mut dialer_events = tn.dialer_events;
    let dialer_network_client = tn.dialer_network_client;
    let listener_peer = tn.listener_peer;
    let mut listener_events = tn.listener_events;
    let listener_sender = tn.listener_network_client;

    let msg = DummyMsg(vec![]);

    // The dialer sends a direct send and listener receives
    let msg_clone = msg.clone();
    let f_dialer = async move {
        dialer_network_client
            .send_to_peer(msg_clone.clone(), listener_peer)
            .unwrap();
        match listener_events.next().await.unwrap() {
            Event::Message(peer_id, msg) => {
                assert_eq!(peer_id, dialer_peer.peer_id());
                assert_eq!(msg, msg_clone);
            },
            event => panic!("Unexpected event {:?}", event),
        }
    };

    // The listener sends a direct send and the dialer receives
    let f_listener = async move {
        listener_sender
            .send_to_peer(msg.clone(), dialer_peer)
            .unwrap();
        match dialer_events.next().await.unwrap() {
            Event::Message(peer_id, incoming_msg) => {
                assert_eq!(peer_id, listener_peer.peer_id());
                assert_eq!(incoming_msg, msg);
            },
            event => panic!("Unexpected event {:?}", event),
        }
    };

    tn.runtime.block_on(join(f_dialer, f_listener));
}

#[test]
fn test_rpc() {
    ::velor_logger::Logger::init_for_testing();
    let tn = setup_network();
    let dialer_peer = tn.dialer_peer;
    let mut dialer_events = tn.dialer_events;
    let dialer_sender = tn.dialer_network_client;
    let listener_peer = tn.listener_peer;
    let mut listener_events = tn.listener_events;
    let listener_sender = tn.listener_network_client;

    let msg = DummyMsg(vec![]);

    // Dialer send rpc request and receives rpc response
    let msg_clone = msg.clone();
    let f_send =
        dialer_sender.send_to_peer_rpc(msg_clone.clone(), Duration::from_secs(10), listener_peer);
    let f_respond = async move {
        match listener_events.next().await.unwrap() {
            Event::RpcRequest(peer_id, msg, _, rs) => {
                assert_eq!(peer_id, dialer_peer.peer_id());
                assert_eq!(msg, msg_clone);
                rs.send(Ok(bcs::to_bytes(&msg).unwrap().into())).unwrap();
            },
            event => panic!("Unexpected event: {:?}", event),
        }
    };

    let (res_msg, _) = tn.runtime.block_on(join(f_send, f_respond));
    assert_eq!(res_msg.unwrap(), msg);

    // Listener send rpc request and receives rpc response
    let msg_clone = msg.clone();
    let f_send =
        listener_sender.send_to_peer_rpc(msg_clone.clone(), Duration::from_secs(10), dialer_peer);
    let f_respond = async move {
        match dialer_events.next().await.unwrap() {
            Event::RpcRequest(peer_id, msg, _, rs) => {
                assert_eq!(peer_id, listener_peer.peer_id());
                assert_eq!(msg, msg_clone);
                rs.send(Ok(bcs::to_bytes(&msg).unwrap().into())).unwrap();
            },
            event => panic!("Unexpected event: {:?}", event),
        }
    };

    let (res_msg, _) = tn.runtime.block_on(join(f_send, f_respond));
    assert_eq!(res_msg.unwrap(), msg);
}
