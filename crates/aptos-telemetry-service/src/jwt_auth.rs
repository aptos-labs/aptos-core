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

use crate::error::ServiceError;
use crate::{context::Context, types::auth::Claims};

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

    if !allow_roles.contains(&claims.peer_role) {
        return Err(reject::custom(ServiceError::forbidden(
            "the peer does not have access to this resource",
        )));
    }

    if claims.epoch == current_epoch && claims.exp > Utc::now().timestamp() as usize {
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

    use std::collections::HashMap;

    use super::super::tests::test_context;
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

    #[tokio::test]
    async fn test_authoize_jwt() {
        let test_context = test_context::new_test_context().await;
        {
            test_context
                .inner
                .validator_cache()
                .write()
                .insert(ChainId::new(25), (10, HashMap::new()));
        }
        let token = create_jwt_token(
            test_context.inner.clone(),
            ChainId::new(25),
            PeerId::random(),
            PeerRole::Validator,
            10,
        )
        .unwrap();
        let result = authorize_jwt(
            token,
            (test_context.inner.clone(), vec![PeerRole::Validator]),
        )
        .await;
        assert!(result.is_ok());

        let token = create_jwt_token(
            test_context.inner.clone(),
            ChainId::new(25),
            PeerId::random(),
            PeerRole::ValidatorFullNode,
            10,
        )
        .unwrap();
        let result = authorize_jwt(token, (test_context.inner, vec![PeerRole::Validator])).await;
        assert!(result.is_err());
        assert_eq!(
            *result.err().unwrap().find::<ServiceError>().unwrap(),
            ServiceError::forbidden("the peer does not have access to this resource",)
        )
    }
}
