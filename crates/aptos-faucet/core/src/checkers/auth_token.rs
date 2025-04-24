// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{CheckerData, CheckerTrait};
use crate::{
    common::{ListManager, ListManagerConfig},
    endpoints::{AptosTapError, RejectionReason, RejectionReasonCode},
    firebase_jwt::X_IS_JWT_HEADER,
};
use anyhow::Result;
use aptos_logger::info;
use async_trait::async_trait;
use poem::http::header::AUTHORIZATION;

pub struct AuthTokenChecker {
    pub manager: ListManager,
}

impl AuthTokenChecker {
    pub fn new(config: ListManagerConfig) -> Result<Self> {
        let manager = ListManager::new(config)?;
        info!(
            "Loaded {} auth tokens into AuthTokenChecker",
            manager.num_items()
        );
        Ok(Self { manager })
    }
}

#[async_trait]
impl CheckerTrait for AuthTokenChecker {
    async fn check(
        &self,
        data: CheckerData,
        _dry_run: bool,
    ) -> Result<Vec<RejectionReason>, AptosTapError> {
        // Don't check if the request has X_IS_JWT_HEADER set.
        if data.headers.contains_key(X_IS_JWT_HEADER) {
            return Ok(vec![]);
        }

        let auth_token = match data
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split_whitespace().nth(1))
        {
            Some(auth_token) => auth_token,
            None => return Ok(vec![RejectionReason::new(
                "Either the Authorization header is missing or it is not in the form of 'Bearer <token>'".to_string(),
                RejectionReasonCode::AuthTokenInvalid,
            )]),
        };
        if self.manager.contains(auth_token) {
            Ok(vec![])
        } else {
            Ok(vec![RejectionReason::new(
                format!(
                    "The given auth token is not allowed by the server: {}",
                    auth_token
                ),
                RejectionReasonCode::AuthTokenInvalid,
            )])
        }
    }

    fn cost(&self) -> u8 {
        2
    }
}
