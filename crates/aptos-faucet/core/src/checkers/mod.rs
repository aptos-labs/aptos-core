// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod auth_token;
mod google_captcha;
mod ip_blocklist;
mod magic_header;
mod memory_ratelimit;
mod redis_ratelimit;
mod tap_captcha;
mod traits;

pub use self::tap_captcha::CaptchaManager;
use self::{
    auth_token::AuthTokenChecker,
    google_captcha::{CaptchaChecker as GoogleCaptchaChecker, GoogleCaptchaCheckerConfig},
    ip_blocklist::IpBlocklistChecker,
    magic_header::{MagicHeaderChecker, MagicHeaderCheckerConfig},
    memory_ratelimit::{MemoryRatelimitChecker, MemoryRatelimitCheckerConfig},
    redis_ratelimit::{RedisRatelimitChecker, RedisRatelimitCheckerConfig},
    tap_captcha::{TapCaptchaChecker, TapCaptchaCheckerConfig},
};
use crate::common::{AuthTokenManagerConfig, IpRangeManagerConfig};
use futures::lock::Mutex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
pub use traits::{Checker, CheckerData, CompleteData};

/// This enum lets us represent all the different checkers in a config. This
/// should only be used at config reading time.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum CheckerConfig {
    /// Requires that an auth token is included in the Authorization header.
    AuthToken(AuthTokenManagerConfig),

    /// Requires a legitimate Google ReCaptcha token.
    GoogleCaptcha(GoogleCaptchaCheckerConfig),

    /// Rejects requests if their IP is in a blocklisted IPrnage.
    IpBlocklist(IpRangeManagerConfig),

    /// Checkers whether a config-defined magic header kv is present.
    MagicHeader(MagicHeaderCheckerConfig),

    /// Basic in memory ratelimiter that allows a single successful request per IP.
    MemoryRatelimit(MemoryRatelimitCheckerConfig),

    /// Ratelimiter that uses Redis.
    RedisRatelimit(RedisRatelimitCheckerConfig),

    /// In-house captcha solution.
    TapCaptcha(TapCaptchaCheckerConfig),
}

impl CheckerConfig {
    pub async fn try_into_boxed_checker(
        self,
        captcha_manager: Arc<Mutex<CaptchaManager>>,
    ) -> Result<Box<dyn Checker>, anyhow::Error> {
        match self {
            Self::AuthToken(config) => Ok(Box::new(AuthTokenChecker::new(config)?)),
            Self::GoogleCaptcha(config) => Ok(Box::new(GoogleCaptchaChecker::new(config)?)),
            Self::IpBlocklist(config) => Ok(Box::new(IpBlocklistChecker::new(config)?)),
            Self::MagicHeader(config) => Ok(Box::new(MagicHeaderChecker::new(config)?)),
            Self::MemoryRatelimit(config) => Ok(Box::new(MemoryRatelimitChecker::new(config))),
            Self::RedisRatelimit(config) => Ok(Box::new(RedisRatelimitChecker::new(config).await?)),
            Self::TapCaptcha(config) => {
                Ok(Box::new(TapCaptchaChecker::new(config, captcha_manager)?))
            },
        }
    }
}
