// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::traits::Bypasser;
use crate::{
    checkers::CheckerData,
    common::{AuthTokenManager, AuthTokenManagerConfig},
};
use anyhow::Result;
use aptos_logger::info;
use async_trait::async_trait;
use poem::http::header::AUTHORIZATION;

pub struct AuthTokenBypasser {
    pub manager: AuthTokenManager,
}

impl AuthTokenBypasser {
    pub fn new(config: AuthTokenManagerConfig) -> Result<Self> {
        let manager = AuthTokenManager::new(config)?;
        info!(
            "Loaded {} auth tokens into AuthTokenBypasser",
            manager.num_auth_tokens()
        );
        Ok(Self { manager })
    }
}

#[async_trait]
impl Bypasser for AuthTokenBypasser {
    async fn request_can_bypass(&self, data: CheckerData) -> Result<bool> {
        let auth_token = match data
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.split_whitespace().nth(1))
        {
            Some(auth_token) => auth_token,
            None => return Ok(false),
        };
        Ok(self.manager.contains_auth_token(auth_token))
    }
}
