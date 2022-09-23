// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::{Peer, PeerRole, PeerSet, RoleType};
use aptos_crypto::noise::{InitiatorHandshakeState, NoiseConfig};
use aptos_crypto::{noise, x25519, Uniform};
use aptos_types::network_address::Protocol::{Dns, Handshake, NoiseIK, Tcp};
use aptos_types::{
    account_address,
    chain_id::ChainId,
    network_address::{DnsName, NetworkAddress},
    PeerId,
};

use serde_json::json;

use crate::context::JsonWebTokenService;
use crate::types::common::NodeType;
use crate::{
    tests::test_context::new_test_context,
    types::auth::{AuthResponse, Claims},
};

fn init(
    peer_role: PeerRole,
) -> (
    rand::rngs::ThreadRng,
    NoiseConfig,
    ChainId,
    PeerId,
    std::collections::HashMap<PeerId, Peer>,
) {
    let mut rng = rand::thread_rng();
    let initiator_static = x25519::PrivateKey::generate(&mut rng);
    let initiator_public_key = initiator_static.public_key();
    let initiator = noise::NoiseConfig::new(initiator_static);
    let chain_id = ChainId::new(21);
    let peer_id = account_address::from_identity_public_key(initiator_public_key);
    let protocols = vec![
        Dns(DnsName::try_from("example.com".to_string()).unwrap()),
        Tcp(1234),
        NoiseIK(initiator_public_key),
        Handshake(0),
    ];
    let addr = NetworkAddress::from_protocols(protocols).unwrap();
    let peer = Peer::from_addrs(peer_role, vec![addr]);
    let mut peer_set = PeerSet::new();
    peer_set.insert(peer_id, peer);

    (rng, initiator, chain_id, peer_id, peer_set)
}

fn init_handshake(
    rng: &mut (impl rand::RngCore + rand::CryptoRng),
    chain_id: ChainId,
    peer_id: PeerId,
    server_public_key: x25519::PublicKey,
    initiator: &NoiseConfig,
) -> (noise::InitiatorHandshakeState, Vec<u8>) {
    // buffer to first noise handshake message
    let mut client_noise_msg = vec![0; noise::handshake_init_msg_len(0)];

    // build the prologue (chain_id | peer_id | public_key)
    const CHAIN_ID_LENGTH: usize = 1;
    const ID_SIZE: usize = CHAIN_ID_LENGTH + PeerId::LENGTH;
    const PROLOGUE_SIZE: usize = CHAIN_ID_LENGTH + PeerId::LENGTH + x25519::PUBLIC_KEY_SIZE;
    let mut prologue = [0; PROLOGUE_SIZE];
    prologue[..CHAIN_ID_LENGTH].copy_from_slice(&[chain_id.id()]);
    prologue[CHAIN_ID_LENGTH..ID_SIZE].copy_from_slice(peer_id.as_ref());
    prologue[ID_SIZE..PROLOGUE_SIZE].copy_from_slice(server_public_key.as_slice());

    // craft first handshake message  (-> e, es, s, ss)
    let initiator_state = initiator
        .initiate_connection(
            rng,
            &prologue,
            server_public_key,
            None,
            &mut client_noise_msg,
        )
        .unwrap();

    (initiator_state, client_noise_msg)
}

fn finish_handshake(
    jwt_service: &JsonWebTokenService,
    initiator: &NoiseConfig,
    initiator_state: InitiatorHandshakeState,
    resp: serde_json::Value,
) -> jsonwebtoken::TokenData<Claims> {
    let resp: AuthResponse = serde_json::from_value(resp).unwrap();

    let (response_payload, _) = initiator
        .finalize_connection(initiator_state, resp.handshake_msg.as_slice())
        .unwrap();

    let jwt = String::from_utf8(response_payload).unwrap();

    jwt_service.decode(&jwt).unwrap()
}

#[tokio::test]
async fn test_auth_validator() {
    let context = new_test_context().await;
    let server_public_key = context.inner.noise_config().public_key();

    let (mut rng, initiator, chain_id, peer_id, peer_set) = init(PeerRole::Validator);

    context
        .inner
        .peers()
        .validators()
        .write()
        .insert(chain_id, (1, peer_set));

    let (initiator_state, client_noise_msg) =
        init_handshake(&mut rng, chain_id, peer_id, server_public_key, &initiator);

    let req = json!({
        "chain_id": chain_id,
        "peer_id": peer_id,
        "role_type": RoleType::Validator,
        "server_public_key": server_public_key,
        "handshake_msg": &client_noise_msg,
    });
    let resp = context.post("/auth", req).await;

    let decoded = finish_handshake(
        context.inner.jwt_service(),
        &initiator,
        initiator_state,
        resp,
    );

    assert_eq!(
        decoded.claims,
        Claims {
            chain_id,
            peer_id,
            node_type: NodeType::Validator,
            epoch: 1,
            exp: decoded.claims.exp,
            iat: decoded.claims.iat
        },
    )
}

#[tokio::test]
async fn test_auth_validatorfullnode() {
    let context = new_test_context().await;
    let server_public_key = context.inner.noise_config().public_key();

    let (mut rng, initiator, chain_id, peer_id, peer_set) = init(PeerRole::ValidatorFullNode);

    context
        .inner
        .peers()
        .validator_fullnodes()
        .write()
        .insert(chain_id, (1, peer_set));

    let (initiator_state, client_noise_msg) =
        init_handshake(&mut rng, chain_id, peer_id, server_public_key, &initiator);

    let req = json!({
        "chain_id": chain_id,
        "peer_id": peer_id,
        "role_type": RoleType::FullNode,
        "server_public_key": server_public_key,
        "handshake_msg": &client_noise_msg,
    });
    let resp = context.post("/auth", req).await;

    let decoded = finish_handshake(
        context.inner.jwt_service(),
        &initiator,
        initiator_state,
        resp,
    );

    assert_eq!(
        decoded.claims,
        Claims {
            chain_id,
            peer_id,
            node_type: NodeType::ValidatorFullNode,
            epoch: 1,
            exp: decoded.claims.exp,
            iat: decoded.claims.iat
        },
    )
}

#[tokio::test]
#[should_panic]
async fn test_auth_wrong_key() {
    let context = new_test_context().await;
    let server_public_key = context.inner.noise_config().public_key();

    let mut rng = rand::thread_rng();
    let initiator_static = x25519::PrivateKey::generate(&mut rng);
    let initiator_static2 = x25519::PrivateKey::generate(&mut rng);
    let initiator_public_key = initiator_static.public_key();
    let initiator = noise::NoiseConfig::new(initiator_static);
    let chain_id = ChainId::new(21);
    let peer_id: PeerId = account_address::from_identity_public_key(initiator_public_key);
    let protocols = vec![
        Dns(DnsName::try_from("example.com".to_string()).unwrap()),
        Tcp(1234),
        NoiseIK(initiator_static2.public_key()),
        Handshake(0),
    ];
    let addr = NetworkAddress::from_protocols(protocols).unwrap();
    let peer = Peer::from_addrs(PeerRole::Validator, vec![addr]);
    let mut peer_set = PeerSet::new();
    peer_set.insert(peer_id, peer);

    context
        .inner
        .peers()
        .validators()
        .write()
        .insert(chain_id, (1, peer_set));

    let (initiator_state, client_noise_msg) =
        init_handshake(&mut rng, chain_id, peer_id, server_public_key, &initiator);

    let req = json!({
        "chain_id": chain_id,
        "peer_id": peer_id,
        "role_type": RoleType::Validator,
        "server_public_key": server_public_key,
        "handshake_msg": client_noise_msg,
    });
    let resp = context.post("/auth", req).await;

    finish_handshake(
        context.inner.jwt_service(),
        &initiator,
        initiator_state,
        resp,
    );
}

#[tokio::test]
async fn test_chain_access() {
    let mut context = new_test_context().await;
    let present_chain_id = ChainId::new(24);
    let missing_chain_id = ChainId::new(32);
    context.inner.chain_set_mut().insert(present_chain_id);

    let resp = context
        .get(&format!("/chain-access/{}", present_chain_id))
        .await;
    assert!(resp.as_bool().unwrap());

    let resp = context
        .get(&format!("/chain-access/{}", missing_chain_id))
        .await;
    assert!(!resp.as_bool().unwrap());
}
