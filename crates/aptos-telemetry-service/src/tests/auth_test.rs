use aptos_config::config::{Peer, PeerRole};
use aptos_crypto::{noise, x25519, Uniform};
use aptos_types::{chain_id::ChainId, PeerId, account_address, network_address::{NetworkAddress, DnsName}};
use serde_json::{json, Value};
use aptos_types::network_address::Protocol::{Dns, Tcp, NoiseIK, Handshake};

use crate::{
    current_function_name,
    tests::test_context::new_test_context,
    types::auth::{AuthResponse},
};

#[tokio::test]
async fn test_auth() {
    let context = new_test_context(current_function_name!());
    let server_public_key = context.context.noise_config().public_key();
    
    let mut rng = rand::thread_rng();
    let initiator_static = x25519::PrivateKey::generate(&mut rng);
    let initiator_public_key = initiator_static.public_key();
    let initiator = noise::NoiseConfig::new(initiator_static);
    let chain_id = ChainId::new(21);
    let peer_id = PeerId::from(account_address::from_identity_public_key(initiator_public_key.clone()));
    let protocols = vec![
        Dns(DnsName::try_from("example.com".to_string()).unwrap()),
        Tcp(1234),
        NoiseIK(initiator_public_key.clone()),
        Handshake(0),
    ];
    let addr = NetworkAddress::from_protocols(protocols).unwrap();
    let peer = Peer::from_addrs(PeerRole::Validator, vec![addr]); 
    
    context.context.validator_cache().validator_store.write().insert((chain_id, peer_id), peer);

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
            &mut rng,
            &prologue,
            server_public_key,
            None,
            &mut client_noise_msg,
        )
        .unwrap();

    let req = json!({
        "chain_id": chain_id,
        "peer_id": peer_id,
        "server_public_key": server_public_key,
        "handshake_msg": client_noise_msg,
    });
    let resp = context.post("/auth", req).await;

    let resp: AuthResponse = serde_json::from_value(resp).unwrap();

    let (_, session) = initiator
        .finalize_connection(initiator_state, resp.handshake_msg.unwrap().as_slice())
        .unwrap();
}

#[tokio::test]
#[should_panic]
async fn test_auth_wrong_key() {
    let context = new_test_context(current_function_name!());
    let server_public_key = context.context.noise_config().public_key();
    
    let mut rng = rand::thread_rng();
    let initiator_static = x25519::PrivateKey::generate(&mut rng);
    let initiator_static2 = x25519::PrivateKey::generate(&mut rng);
    let initiator_public_key = initiator_static.public_key();
    let initiator = noise::NoiseConfig::new(initiator_static);
    let chain_id = ChainId::new(21);
    let peer_id = PeerId::from(account_address::from_identity_public_key(initiator_public_key.clone()));
    let protocols = vec![
        Dns(DnsName::try_from("example.com".to_string()).unwrap()),
        Tcp(1234),
        NoiseIK(initiator_static2.public_key()),
        Handshake(0),
    ];
    let addr = NetworkAddress::from_protocols(protocols).unwrap();
    let peer = Peer::from_addrs(PeerRole::Validator, vec![addr]); 
    
    context.context.validator_cache().validator_store.write().insert((chain_id, peer_id), peer);

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
            &mut rng,
            &prologue,
            server_public_key,
            None,
            &mut client_noise_msg,
        )
        .unwrap();

    let req = json!({
        "chain_id": chain_id,
        "peer_id": peer_id,
        "server_public_key": server_public_key,
        "handshake_msg": client_noise_msg,
    });
    let resp = context.post("/auth", req).await;

    let resp: AuthResponse = serde_json::from_value(resp).unwrap();

    let (_, session) = initiator
        .finalize_connection(initiator_state, resp.handshake_msg.unwrap().as_slice())
        .unwrap();
}