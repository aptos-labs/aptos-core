// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

mod auth_token;
mod ip_allowlist;

use self::{auth_token::AuthTokenBypasser, ip_allowlist::IpAllowlistBypasser};
use crate::{
    checkers::CheckerData,
    common::{IpRangeManagerConfig, ListManagerConfig},
};
use anyhow::Result;
use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use serde::{Deserialize, Serialize};

/// This trait defines something that checks whether a given request should
/// skip all the checkers and storage, for example an IP allowlist.
#[async_trait]
#[enum_dispatch]
pub trait BypasserTrait: Sync + Send + 'static {
    /// Returns true if the request should be allowed to bypass all checkers
    /// and storage.
    async fn request_can_bypass(&self, data: CheckerData) -> Result<bool>;
}

/// This enum lets us represent all the different Bypassers in a config.
/// This should only be used at config reading time.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum BypasserConfig {
    AuthToken(ListManagerConfig),
    IpAllowlist(IpRangeManagerConfig),
}

impl BypasserConfig {
    pub fn build(self) -> Result<Bypasser> {
        Ok(match self {
            BypasserConfig::AuthToken(config) => Bypasser::from(AuthTokenBypasser::new(config)?),

            BypasserConfig::IpAllowlist(config) => {
                Bypasser::from(IpAllowlistBypasser::new(config)?)
            },
        })
    }
}

/// This enum has as its variants all possible implementations of BypasserTrait.
#[enum_dispatch(BypasserTrait)]
pub enum Bypasser {
    AuthTokenBypasser,
    IpAllowlistBypasser,
}
