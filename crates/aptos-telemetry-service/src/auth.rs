// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    context::Context,
    debug, error,
    errors::{json_rejection_to_service_error, AuthError, ServiceError, ServiceErrorCode},
    jwt_auth::{authorize_jwt, create_jwt_token, jwt_from_header},
    types::{
        auth::{AuthRequest, AuthResponse, Claims},
        common::NodeType,
    },
    warn,
};
use aptos_config::config::{PeerRole, RoleType};
use aptos_crypto::{noise, x25519};
use aptos_types::{chain_id::ChainId, PeerId};
use axum::{
    extract::{rejection::JsonRejection, Extension, Json, Path},
    http::{header::AUTHORIZATION, HeaderMap},
};

pub async fn post_auth(
    Extension(context): Extension<Context>,
    body: Result<Json<AuthRequest>, JsonRejection>,
) -> Result<Json<AuthResponse>, ServiceError> {
    let Json(body) = body.map_err(json_rejection_to_service_error)?;
    handle_auth(context, body).await.map(Json)
}

pub async fn handle_auth(
    context: Context,
    body: AuthRequest,
) -> Result<AuthResponse, ServiceError> {
    debug!("received auth request: {:?}", body);

    let client_init_message = &body.handshake_msg;

    if body.server_public_key != context.noise_config().public_key() {
        return Err(ServiceError::bad_request(ServiceErrorCode::AuthError(
            AuthError::InvalidServerPublicKey,
            body.chain_id,
        )));
    }

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
            ServiceError::bad_request(ServiceErrorCode::AuthError(
                AuthError::NoiseHandshakeError(e),
                body.chain_id,
            ))
        })?;

    let cache = if body.role_type == RoleType::Validator {
        context.peers().validators()
    } else {
        context.peers().validator_fullnodes()
    };

    let (epoch, peer_role) = match cache.read().get(&body.chain_id) {
        Some((epoch, peer_set)) => match peer_set.get(&body.peer_id) {
            Some(peer) => {
                let remote_public_key = &remote_public_key;
                if !peer.keys.contains(remote_public_key) {
                    warn!("peer found in peer set but public_key is not found. request body: {}, role_type: {}, peer_id: {}, received public_key: {}", body.chain_id, body.role_type, body.peer_id, remote_public_key);
                    return Err(ServiceError::forbidden(ServiceErrorCode::AuthError(
                        AuthError::PeerPublicKeyNotFound,
                        body.chain_id,
                    )));
                }
                Ok((*epoch, peer.role))
            },
            None => {
                let derived_remote_peer_id =
                    aptos_types::account_address::from_identity_public_key(remote_public_key);
                if derived_remote_peer_id != body.peer_id {
                    return Err(ServiceError::forbidden(ServiceErrorCode::AuthError(
                        AuthError::PublicKeyMismatch,
                        body.chain_id,
                    )));
                } else {
                    Ok((*epoch, PeerRole::Unknown))
                }
            },
        },
        None => {
            warn!(
                "Validator set unavailable for Chain ID {}. Rejecting request.",
                body.chain_id
            );
            Err(ServiceError::unauthorized(ServiceErrorCode::AuthError(
                AuthError::ValidatorSetUnavailable,
                body.chain_id,
            )))
        },
    }?;

    let node_type = match peer_role {
        PeerRole::Validator => NodeType::Validator,
        PeerRole::ValidatorFullNode => NodeType::ValidatorFullNode,
        PeerRole::Unknown => match body.role_type {
            RoleType::Validator => NodeType::UnknownValidator,
            RoleType::FullNode => context
                .peers()
                .public_fullnodes()
                .get(&body.chain_id)
                .and_then(|peer_set| {
                    if peer_set.contains_key(&body.peer_id) {
                        Some(NodeType::PublicFullNode)
                    } else {
                        None
                    }
                })
                .unwrap_or(NodeType::UnknownFullNode),
        },
        _ => NodeType::Unknown,
    };

    let token = create_jwt_token(
        context.jwt_service(),
        body.chain_id,
        body.peer_id,
        node_type,
        epoch,
        body.run_uuid,
    )
    .map_err(|e| {
        error!("unable to create jwt token: {}", e);
        ServiceError::internal(ServiceErrorCode::AuthError(
            AuthError::from(e),
            body.chain_id,
        ))
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

    Ok(AuthResponse {
        handshake_msg: server_response,
    })
}

/// Authorize a JWT from request headers for the given node roles.
pub async fn authorize_request(
    context: &Context,
    headers: &HeaderMap,
    allow_roles: &[NodeType],
) -> Result<Claims, ServiceError> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let token = jwt_from_header(auth_header).await?;
    authorize_jwt(token, context.clone(), allow_roles.to_vec()).await
}

pub async fn get_chain_access(
    Extension(context): Extension<Context>,
    Path(chain_id): Path<ChainId>,
) -> Result<Json<bool>, ServiceError> {
    let present = context.chain_set().contains(&chain_id);
    Ok(Json(present))
}
