// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::PeerRole;
use aptos_logger::error;
use aptos_types::{chain_id::ChainId, PeerId};
use chrono::Utc;
use jsonwebtoken::{decode, encode, errors::Error, Algorithm, Header, Validation};
use warp::{
    http::header::{HeaderMap, HeaderValue, AUTHORIZATION},
    reject, Rejection,
};

use crate::{context::Context, error::ServiceError, types::auth::Claims};

const BEARER: &str = "BEARER ";

pub fn create_jwt_token(
    context: Context,
    chain_id: ChainId,
    peer_id: PeerId,
    peer_role: PeerRole,
    epoch: u64,
) -> Result<String, Error> {
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
    encode(&header, &claims, &context.jwt_encoding_key)
}

pub async fn authorize_jwt(
    token: String,
    (context, allow_roles): (Context, Vec<PeerRole>),
) -> anyhow::Result<Claims, Rejection> {
    let decoded = decode::<Claims>(
        &token,
        &context.jwt_decoding_key,
        &Validation::new(Algorithm::HS512),
    )
    .map_err(|e| {
        error!("unable to authorize jwt token: {}", e);
        reject::custom(ServiceError::unauthorized("invalid authorization token"))
    })?;

    let claims = decoded.claims;

    let current_epoch = match context.validator_cache().read().get(&claims.chain_id) {
        Some(info) => info.0,
        None => {
            return Err(reject::custom(ServiceError::unauthorized(
                "expired authorization token",
            )));
        }
    };

    if allow_roles.contains(&claims.peer_role)
        && claims.epoch == current_epoch
        && claims.exp > Utc::now().timestamp() as usize
    {
        Ok(claims)
    } else {
        Err(reject::custom(ServiceError::unauthorized(
            "expired authorization token",
        )))
    }
}

pub async fn jwt_from_header(headers: HeaderMap<HeaderValue>) -> anyhow::Result<String, Rejection> {
    let header = match headers.get(AUTHORIZATION) {
        Some(v) => v,
        None => {
            return Err(reject::custom(ServiceError::unauthorized(
                "no authorization header present",
            )))
        }
    };
    let auth_header = match std::str::from_utf8(header.as_bytes()) {
        Ok(v) => v,
        Err(_) => return Err(reject::reject()),
    };
    let auth_header = auth_header.split(',').next().unwrap_or_default();
    if !auth_header
        .get(..BEARER.len())
        .unwrap_or_default()
        .eq_ignore_ascii_case(BEARER)
    {
        return Err(reject::custom(ServiceError::unauthorized(
            "invalid authorization header",
        )));
    }
    Ok(auth_header
        .get(BEARER.len()..)
        .unwrap_or_default()
        .to_owned())
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn jwt_from_header_valid_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Bearer token".parse().unwrap());
        assert_eq!(jwt_from_header(headers).await.unwrap(), "token");

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "bearer token".parse().unwrap());
        assert_eq!(jwt_from_header(headers).await.unwrap(), "token");

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "BEARER token".parse().unwrap());
        assert_eq!(jwt_from_header(headers).await.unwrap(), "token");
    }

    #[tokio::test]
    async fn jwt_from_header_invalid_bearer() {
        let headers = HeaderMap::new();
        let jwt = jwt_from_header(headers).await;
        assert!(jwt.is_err());

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Bear token".parse().unwrap());
        let jwt = jwt_from_header(headers).await;
        assert!(jwt.is_err());

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "".parse().unwrap());
        let jwt = jwt_from_header(headers).await;
        assert!(jwt.is_err());

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Bear".parse().unwrap());
        let jwt = jwt_from_header(headers).await;
        assert!(jwt.is_err());

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "BEARER: token".parse().unwrap());
        let jwt = jwt_from_header(headers).await;
        assert!(jwt.is_err());
    }
}
