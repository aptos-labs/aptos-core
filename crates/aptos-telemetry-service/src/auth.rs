// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::context::Context;
use crate::errors::{AuthError, ServiceError, ServiceErrorCode};
use crate::jwt_auth::{authorize_jwt, create_jwt_token, jwt_from_header};
use crate::types::auth::{AuthRequest, AuthResponse, Claims};
use crate::types::common::NodeType;
use crate::{debug, error, warn};

use anyhow::Result;
use reqwest::header::AUTHORIZATION;
use warp::filters::BoxedFilter;
use warp::{reject, Filter, Rejection};
use warp::{reply, Reply};

use aptos_config::config::{PeerRole, RoleType};
use aptos_crypto::{noise, x25519};
use aptos_types::chain_id::ChainId;
use aptos_types::PeerId;

pub fn auth(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("auth")
        .and(warp::post())
        .and(context.filter())
        .and(warp::body::json())
        .and_then(handle_auth)
        .boxed()
}

pub async fn handle_auth(context: Context, body: AuthRequest) -> Result<impl Reply, Rejection> {
    debug!("received auth request: {:?}", body);

    let client_init_message = &body.handshake_msg;

    // Check whether the client (validator) is using the correct server's public key, which the
    // client includes in its request body.
    // This is useful for returning a refined error response to the client if it is using an
    // invalid server public key.
    if body.server_public_key != context.noise_config().public_key() {
        return Err(reject::custom(ServiceError::bad_request(
            ServiceErrorCode::AuthError(AuthError::InvalidServerPublicKey, body.chain_id),
        )));
    }

    // build the prologue (chain_id | peer_id | public_key)
    const CHAIN_ID_LENGTH: usize = 1;
    const ID_SIZE: usize = CHAIN_ID_LENGTH + PeerId::LENGTH;
    const PROLOGUE_SIZE: usize = CHAIN_ID_LENGTH + PeerId::LENGTH + x25519::PUBLIC_KEY_SIZE;
    let mut prologue = [0; PROLOGUE_SIZE];
    prologue[..CHAIN_ID_LENGTH].copy_from_slice(&[body.chain_id.id()]);
    prologue[CHAIN_ID_LENGTH..ID_SIZE].copy_from_slice(body.peer_id.as_ref());
    prologue[ID_SIZE..PROLOGUE_SIZE].copy_from_slice(body.server_public_key.as_slice());

    let (remote_public_key, handshake_state, _payload) = context
        .noise_config()
        .parse_client_init_message(&prologue, client_init_message)
        .map_err(|e| {
            debug!("error performing noise handshake: {}", e);
            reject::custom(ServiceError::bad_request(ServiceErrorCode::AuthError(
                AuthError::NoiseHandshakeError(e),
                body.chain_id,
            )))
        })?;

    let cache = if body.role_type == RoleType::Validator {
        context.peers().validators()
    } else {
        context.peers().validator_fullnodes()
    };

    let (epoch, peer_role) = match cache.read().get(&body.chain_id) {
        Some((epoch, peer_set)) => {
            match peer_set.get(&body.peer_id) {
                Some(peer) => {
                    let remote_public_key = &remote_public_key;
                    if !peer.keys.contains(remote_public_key) {
                        warn!("peer found in peer set but public_key is not found. request body: {}, role_type: {}, peer_id: {}, received public_key: {}", body.chain_id, body.role_type, body.peer_id, remote_public_key);
                        return Err(reject::custom(ServiceError::forbidden(
                            ServiceErrorCode::AuthError(
                                AuthError::PeerPublicKeyNotFound,
                                body.chain_id,
                            ),
                        )));
                    }
                    Ok((*epoch, peer.role))
                }
                None => {
                    // if not, verify that their peerid is constructed correctly from their public key
                    let derived_remote_peer_id =
                        aptos_types::account_address::from_identity_public_key(remote_public_key);
                    if derived_remote_peer_id != body.peer_id {
                        return Err(reject::custom(ServiceError::forbidden(
                            ServiceErrorCode::AuthError(
                                AuthError::PublicKeyMismatch,
                                body.chain_id,
                            ),
                        )));
                    } else {
                        Ok((*epoch, PeerRole::Unknown))
                    }
                }
            }
        }
        None => {
            warn!(
                "Validator set unavailable for Chain ID {}. Rejecting request.",
                body.chain_id
            );
            Err(reject::custom(ServiceError::unauthorized(
                ServiceErrorCode::AuthError(AuthError::ValidatorSetUnavailable, body.chain_id),
            )))
        }
    }?;

    let node_type = match peer_role {
        PeerRole::Validator => NodeType::Validator,
        PeerRole::ValidatorFullNode => NodeType::ValidatorFullNode,
        PeerRole::Unknown => context
            .peers()
            .public_fullnodes()
            .get(&body.chain_id)
            .map(|peer_set| {
                if peer_set.contains_key(&body.peer_id) {
                    NodeType::PublicFullNode
                } else {
                    NodeType::Unknown
                }
            })
            .unwrap_or(NodeType::Unknown),
        _ => NodeType::Unknown,
    };

    let token = create_jwt_token(
        context.jwt_service(),
        body.chain_id,
        body.peer_id,
        node_type,
        epoch,
    )
    .map_err(|e| {
        error!("unable to create jwt token: {}", e);
        reject::custom(ServiceError::internal(ServiceErrorCode::AuthError(
            AuthError::from(e),
            body.chain_id,
        )))
    })?;

    let mut rng = rand::rngs::OsRng;
    let response_payload = token.as_bytes();
    let mut server_response = vec![0u8; noise::handshake_resp_msg_len(response_payload.len())];
    context
        .noise_config()
        .respond_to_client(
            &mut rng,
            handshake_state,
            Some(response_payload),
            &mut server_response,
        )
        .map_err(|e| {
            error!("unable to complete handshake {}", e);
            ServiceError::internal(ServiceErrorCode::AuthError(
                AuthError::NoiseHandshakeError(e),
                body.chain_id,
            ))
        })?;

    Ok(reply::json(&AuthResponse {
        handshake_msg: server_response,
    }))
}

pub fn with_auth(
    context: Context,
    roles: Vec<NodeType>,
) -> impl Filter<Extract = (Claims,), Error = Rejection> + Clone {
    warp::header::optional(AUTHORIZATION.as_str())
        .and_then(jwt_from_header)
        .and(
            warp::any()
                .map(move || (context.clone(), roles.clone()))
                .untuple_one(),
        )
        .and_then(authorize_jwt)
}

pub fn check_chain_access(context: Context) -> BoxedFilter<(impl Reply,)> {
    warp::path!("chain-access" / ChainId)
        .and(warp::get())
        .and(context.filter())
        .and_then(handle_check_chain_access)
        .boxed()
}

async fn handle_check_chain_access(
    chain_id: ChainId,
    context: Context,
) -> Result<impl Reply, Rejection> {
    let present = context.chain_set().contains(&chain_id);
    Ok(reply::json(&present))
}
