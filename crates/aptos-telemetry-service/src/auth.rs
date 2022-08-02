use crate::context::Context;
use crate::types::auth::{AuthRequest, AuthResponse, Claims};
use aptos_config::config::{Peer, PeerRole};
use aptos_crypto::{noise, x25519};
use aptos_types::chain_id::ChainId;
use aptos_types::PeerId;
use chrono::Utc;
use jsonwebtoken::{errors::Error, Algorithm, EncodingKey, Header};
use warp::filters::BoxedFilter;
use warp::{reject, reply, Filter, Rejection, Reply};

pub fn auth(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("auth")
        .and(warp::post())
        .and(context.filter())
        .and(warp::body::json())
        .and_then(handle_auth)
        .boxed()
}

pub async fn handle_auth(
    context: Context,
    body: AuthRequest,
) -> anyhow::Result<impl Reply, Rejection> {
    let client_init_message = &body.handshake_msg;

    // verify that this is indeed our public key
    if body.server_public_key != context.noise_config().public_key() {
        return Err(reject::reject());
    }

    // build the prologue (chain_id | peer_id | public_key)
    const CHAIN_ID_LENGTH: usize = 1;
    const ID_SIZE: usize = CHAIN_ID_LENGTH + PeerId::LENGTH;
    const PROLOGUE_SIZE: usize = CHAIN_ID_LENGTH + PeerId::LENGTH + x25519::PUBLIC_KEY_SIZE;
    let mut prologue = [0; PROLOGUE_SIZE];
    prologue[..CHAIN_ID_LENGTH].copy_from_slice(&[body.chain_id.id()]);
    prologue[CHAIN_ID_LENGTH..ID_SIZE].copy_from_slice(body.peer_id.as_ref());
    prologue[ID_SIZE..PROLOGUE_SIZE].copy_from_slice(body.server_public_key.as_slice());

    let (remote_public_key, handshake_state, payload) = context
        .noise_config()
        .parse_client_init_message(&prologue, client_init_message)
        .map_err(|_| reject::reject())?;

    let epoch = 0;
    let peer_role = match context.validator_cache().validator_store.read().get(&(body.chain_id, body.peer_id)) {
        Some(peer) => authenticate_inbound(peer, &remote_public_key),
        None => {
            // if not, verify that their peerid is constructed correctly from their public key
            let derived_remote_peer_id =
                aptos_types::account_address::from_identity_public_key(remote_public_key);
            if derived_remote_peer_id != body.peer_id {
                Err(reject::reject())
            } else {
                Ok(PeerRole::Unknown)
            }
        }
    }?;

    let token = create_jwt(body.chain_id, body.peer_id, peer_role, epoch)
        .map_err(|_| reject::reject())?;

    let mut rng = rand::rngs::OsRng;
    let response_payload = token.as_bytes();
    let mut server_response = vec![0u8; noise::handshake_resp_msg_len(response_payload.len())];
    context
        .noise_config()
        .respond_to_client(&mut rng, handshake_state, Some(response_payload), &mut server_response)
        .map_err(|_| reject::reject())?;

    Ok(reply::json(&AuthResponse {
        handshake_msg: Some(server_response.to_owned()),
    }))
}

pub fn create_jwt(chain_id: ChainId, peer_id: PeerId, peer_role: PeerRole, epoch: u64) -> Result<String, Error> {
    let issued = Utc::now().timestamp();
    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::minutes(60))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        chain_id,
        peer_id,
        peer_role,
        epoch,
        exp: expiration as usize,
        iat: issued as usize,
    };
    let header = Header::new(Algorithm::HS512);
    jsonwebtoken::encode(&header, &claims, &EncodingKey::from_secret(b"open_to_the_world_secret"))
}

fn authenticate_inbound(
    peer: &Peer,
    remote_public_key: &x25519::PublicKey,
) -> Result<PeerRole, Rejection> {
    if !peer.keys.contains(remote_public_key) {
        return Err(reject::reject());
    }
    Ok(peer.role)
}
