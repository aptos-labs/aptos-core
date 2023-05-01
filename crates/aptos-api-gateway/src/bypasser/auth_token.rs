// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::BypasserTrait;
use crate::{
    checkers::CheckerData,
    common::{ListManager, ListManagerConfig},
};
use anyhow::Result;
use aptos_logger::info;
use async_trait::async_trait;
use poem::http::header::AUTHORIZATION;

pub struct AuthTokenBypasser {
    pub manager: ListManager,
}

impl AuthTokenBypasser {
    pub fn new(config: ListManagerConfig) -> Result<Self> {
        let manager = ListManager::new(config)?;
        info!(
            "Loaded {} auth tokens into AuthTokenBypasser",
            manager.num_items()
        );
        Ok(Self { manager })
    }
}

#[async_trait]
impl BypasserTrait for AuthTokenBypasser {
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
        Ok(self.manager.contains(auth_token))
    }
}
