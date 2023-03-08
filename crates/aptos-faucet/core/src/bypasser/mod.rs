// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod auth_token;
mod ip_allowlist;
mod traits;

pub use self::traits::Bypasser;
use self::{auth_token::AuthTokenBypasser, ip_allowlist::IpAllowlistBypasser};
use crate::common::{AuthTokenManagerConfig, IpRangeManagerConfig};
use serde::{Deserialize, Serialize};

/// This enum lets us represent all the different Bypassers in a config.
/// This should only be used at config reading time.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum BypasserConfig {
    AuthToken(AuthTokenManagerConfig),
    IpAllowlist(IpRangeManagerConfig),
}

impl BypasserConfig {
    pub fn try_into_boxed_bypasser(self) -> Result<Box<dyn Bypasser>, anyhow::Error> {
        match self {
            Self::AuthToken(config) => Ok(Box::new(AuthTokenBypasser::new(config)?)),
            Self::IpAllowlist(config) => Ok(Box::new(IpAllowlistBypasser::new(config)?)),
        }
    }
}
