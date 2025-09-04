// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    context::{Context, JsonWebTokenService},
    error,
    errors::{JwtAuthError, ServiceError},
    types::{auth::Claims, common::NodeType},
};
use velor_types::{chain_id::ChainId, PeerId};
use chrono::Utc;
use jsonwebtoken::{errors::Error, TokenData};
use uuid::Uuid;
use warp::{reject, Rejection};

const BEARER: &str = "BEARER ";

pub fn create_jwt_token(
    jwt_service: &JsonWebTokenService,
    chain_id: ChainId,
    peer_id: PeerId,
    node_type: NodeType,
    epoch: u64,
    uuid: Uuid,
) -> Result<String, Error> {
    let issued = Utc::now().timestamp();
    let expiration = Utc::now()
        .checked_add_signed(chrono::Duration::minutes(60))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        chain_id,
        peer_id,
        node_type,
        epoch,
        exp: expiration as usize,
        iat: issued as usize,
        run_uuid: uuid,
    };
    jwt_service.encode(claims)
}

pub async fn authorize_jwt(
    token: String,
    context: Context,
    allow_roles: Vec<NodeType>,
) -> anyhow::Result<Claims, Rejection> {
    let decoded: TokenData<Claims> = context.jwt_service().decode(&token).map_err(|e| {
        error!("unable to authorize jwt token: {}", e);
        reject::custom(ServiceError::unauthorized(
            JwtAuthError::InvalidAuthToken.into(),
        ))
    })?;
    let claims = decoded.claims;

    let current_epoch = match context.peers().validators().read().get(&claims.chain_id) {
        Some(info) => info.0,
        None => {
            return Err(reject::custom(ServiceError::unauthorized(
                JwtAuthError::ExpiredAuthToken.into(),
            )));
        },
    };

    if !allow_roles.contains(&claims.node_type) {
        return Err(reject::custom(ServiceError::forbidden(
            JwtAuthError::AccessDenied.into(),
        )));
    }

    if claims.epoch == current_epoch && claims.exp > Utc::now().timestamp() as usize {
        Ok(claims)
    } else {
        Err(reject::custom(ServiceError::unauthorized(
            JwtAuthError::ExpiredAuthToken.into(),
        )))
    }
}

pub async fn jwt_from_header(auth_header: Option<String>) -> anyhow::Result<String, Rejection> {
    let auth_header = match auth_header {
        Some(v) => v,
        None => {
            return Err(reject::custom(ServiceError::unauthorized(
                JwtAuthError::from("bearer token missing".to_owned()).into(),
            )))
        },
    };
    let auth_header = auth_header.split(',').next().unwrap_or_default();
    if !auth_header
        .get(..BEARER.len())
        .unwrap_or_default()
        .eq_ignore_ascii_case(BEARER)
    {
        return Err(reject::custom(ServiceError::unauthorized(
            JwtAuthError::from("malformed bearer token".to_owned()).into(),
        )));
    }
    Ok(auth_header
        .get(BEARER.len()..)
        .unwrap_or_default()
        .to_owned())
}

#[cfg(test)]
mod tests {

    use super::{super::tests::test_context, *};
    use std::collections::HashMap;
    use warp::hyper::StatusCode;

    #[tokio::test]
    async fn jwt_from_header_valid_bearer() {
        assert_eq!(
            jwt_from_header(Some("Bearer token".into())).await.unwrap(),
            "token"
        );

        assert_eq!(
            jwt_from_header(Some("bearer token".into())).await.unwrap(),
            "token"
        );

        assert_eq!(
            jwt_from_header(Some("BEARER token".into())).await.unwrap(),
            "token"
        );
    }

    #[tokio::test]
    async fn jwt_from_header_invalid_bearer() {
        let jwt = jwt_from_header(None).await;
        assert!(jwt.is_err());

        let jwt = jwt_from_header(Some("Bear token".into())).await;
        assert!(jwt.is_err());

        let jwt = jwt_from_header(Some("".into())).await;
        assert!(jwt.is_err());

        let jwt = jwt_from_header(Some("Bear".into())).await;
        assert!(jwt.is_err());

        let jwt = jwt_from_header(Some("BEARER: token".into())).await;
        assert!(jwt.is_err());
    }

    #[tokio::test]
    async fn test_authoize_jwt() {
        let test_context = test_context::new_test_context().await;
        {
            test_context
                .inner
                .peers()
                .validators()
                .write()
                .insert(ChainId::new(25), (10, HashMap::new()));
        }
        let token = create_jwt_token(
            test_context.inner.jwt_service(),
            ChainId::new(25),
            PeerId::random(),
            NodeType::Validator,
            10,
            Uuid::default(),
        )
        .unwrap();
        let result =
            authorize_jwt(token, test_context.inner.clone(), vec![NodeType::Validator]).await;
        assert!(result.is_ok());

        let token = create_jwt_token(
            test_context.inner.jwt_service(),
            ChainId::new(25),
            PeerId::random(),
            NodeType::ValidatorFullNode,
            10,
            Uuid::default(),
        )
        .unwrap();
        let result = authorize_jwt(token, test_context.inner, vec![NodeType::Validator]).await;
        assert!(result.is_err());

        let rejection = result.err().unwrap();
        let service_error = rejection.find::<ServiceError>().unwrap();
        assert_eq!(service_error.http_status_code(), StatusCode::FORBIDDEN);
        assert_eq!(
            service_error.error_as_string(),
            "authorization error: access denied to this resource"
        );
    }
}
