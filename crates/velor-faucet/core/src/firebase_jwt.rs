// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::endpoints::{VelorTapError, VelorTapErrorCode};
use anyhow::Result;
use firebase_token::JwkAuth;
use poem::http::{header::AUTHORIZATION, HeaderMap};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const X_IS_JWT_HEADER: &str = "x-is-jwt";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FirebaseJwtVerifierConfig {
    pub identity_platform_gcp_project: String,
}

/// This verifies that the value in the Authorization header is a valid Firebase JWT.
/// Since we already have achecker that looks for API keys using the Authorization
/// header, we mandate that a `x-is-jwt` header is present as well.
pub struct FirebaseJwtVerifier {
    pub jwt_verifier: JwkAuth,
}

impl FirebaseJwtVerifier {
    pub async fn new(config: FirebaseJwtVerifierConfig) -> Result<Self> {
        let jwt_verifier = JwkAuth::new(config.identity_platform_gcp_project).await;
        Ok(Self { jwt_verifier })
    }

    /// First, we mandate that the caller indicated that they're including a JWT by
    /// checking for the presence of X_IS_JWT_HEADER. If they didn't include this
    /// header, we reject them immediately. We need this because we already have a
    /// checker that looks for API keys using the Authorization header, and we want
    /// to differentiate these two cases.
    ///
    /// If they did include X_IS_JWT_HEADER and the Authorization header was present
    /// and well-formed, we extract the token from the Authorization header and verify
    /// it with Firebase. If the token is invalid, we reject them. If it is valid, we
    /// return the UID (from the sub field).
    pub async fn validate_jwt(&self, headers: Arc<HeaderMap>) -> Result<String, VelorTapError> {
        let auth_token = jwt_sub(headers)?;

        let verify = self.jwt_verifier.verify::<JwtClaims>(&auth_token);
        let token_data = match verify.await {
            Some(token_data) => token_data,
            None => {
                return Err(VelorTapError::new(
                    "Failed to verify JWT token".to_string(),
                    VelorTapErrorCode::AuthTokenInvalid,
                ));
            },
        };
        let claims = token_data.claims;

        if !claims.email_verified {
            return Err(VelorTapError::new(
                "The JWT token is not verified".to_string(),
                VelorTapErrorCode::AuthTokenInvalid,
            ));
        }

        Ok(claims.sub)
    }
}

/// Returns the sub field from a JWT if it is present (the Firebase UID).
/// The X_IS_JWT_HEADER must be present and the value must be "true".
pub fn jwt_sub(headers: Arc<HeaderMap>) -> Result<String, VelorTapError> {
    let is_jwt = headers
        .get(X_IS_JWT_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("true"))
        .ok_or_else(|| {
            VelorTapError::new(
                format!(
                    "The {} header must be present and set to 'true'",
                    X_IS_JWT_HEADER
                ),
                VelorTapErrorCode::AuthTokenInvalid,
            )
        })?;

    if !is_jwt {
        return Err(VelorTapError::new(
            format!("The {} header must be set to 'true'", X_IS_JWT_HEADER),
            VelorTapErrorCode::AuthTokenInvalid,
        ));
    }

    match headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split_whitespace().nth(1))
    {
        Some(auth_token) => Ok(auth_token.to_string()),
        None => Err(VelorTapError::new(
            "Either the Authorization header is missing or it is not in the form of 'Bearer <token>'".to_string(),
            VelorTapErrorCode::AuthTokenInvalid,
        )),
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct JwtClaims {
    pub aud: String,
    pub exp: i64,
    pub iss: String,
    pub sub: String,
    pub iat: i64,
    pub email: String,
    pub email_verified: bool,
}
