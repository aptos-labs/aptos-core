// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

mod auth_token;
mod google_captcha;
mod ip_blocklist;
mod magic_header;
mod memory_ratelimit;
mod redis_ratelimit;
mod referer_blocklist;
mod tap_captcha;

pub use self::tap_captcha::CaptchaManager;
use self::{
    auth_token::AuthTokenChecker,
    google_captcha::{CaptchaChecker as GoogleCaptchaChecker, GoogleCaptchaCheckerConfig},
    ip_blocklist::IpBlocklistChecker,
    magic_header::{MagicHeaderChecker, MagicHeaderCheckerConfig},
    memory_ratelimit::{MemoryRatelimitChecker, MemoryRatelimitCheckerConfig},
    redis_ratelimit::{RedisRatelimitChecker, RedisRatelimitCheckerConfig},
    referer_blocklist::RefererBlocklistChecker,
    tap_captcha::{TapCaptchaChecker, TapCaptchaCheckerConfig},
};
use crate::{
    common::{IpRangeManagerConfig, ListManagerConfig},
    endpoints::{VelorTapError, RejectionReason},
};
use anyhow::Result;
use velor_sdk::types::account_address::AccountAddress;
use async_trait::async_trait;
use enum_dispatch::enum_dispatch;
use futures::lock::Mutex;
use poem::http::HeaderMap;
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, sync::Arc};
use tokio::task::JoinSet;

/// Implementers of this trait are responsible for checking something about the
/// request, and if it doesn't look valid, returning a list of rejection reasons
/// explaining why. It may also do something extra after the funding happened
/// if there is something to clean up afterwards.
#[async_trait]
#[enum_dispatch]
pub trait CheckerTrait: Sync + Send + 'static {
    /// Returns a list of rejection reasons for the request, if any. If dry_run
    /// is set, if this Checker would store anything based on the request, it
    /// instead will not. This is useful for the is_eligible endpoint.
    async fn check(
        &self,
        data: CheckerData,
        dry_run: bool,
    ) -> Result<Vec<RejectionReason>, VelorTapError>;

    /// If the Checker wants to do anything after the funding has completed, it
    /// may do so in this function. For example, for the storage Checkers, this
    /// function is responsible for marking a request in storage as complete,
    /// in both success and failure cases. It can also store additional metadata
    /// included in CompleteData that we might have from the call to the Funder.
    /// No dry_run flag for this, because we should never need to run this in
    /// dry_run mode.
    async fn complete(&self, _data: CompleteData) -> Result<(), VelorTapError> {
        Ok(())
    }

    /// Aribtrary cost, where lower is less cost. We use these to determine the
    /// order we run checkers.
    fn cost(&self) -> u8;

    /// This function will be called once at startup. In it, the trait implementation
    /// should spawn any periodic tasks that it wants and return handles to them.
    /// If tasks want to signal that there is an issue, all they have to do is return.
    /// If the task wants to tolerate some errors, e.g. only cause the process to die
    /// if the task has failed n times, it must handle that itself and only return
    /// when it wants this to happen.
    // Sadly we can't use ! here yet: https://github.com/rust-lang/rust/issues/35121.
    fn spawn_periodic_tasks(&self, _join_set: &mut JoinSet<anyhow::Result<()>>) {}
}

/// This enum lets us represent all the different checkers in a config. This
/// should only be used at config reading time.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum CheckerConfig {
    /// Requires that an auth token is included in the Authorization header.
    AuthToken(ListManagerConfig),

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

    /// Rejects requests if their Referer is blocklisted.
    RefererBlocklist(ListManagerConfig),

    /// In-house captcha solution.
    TapCaptcha(TapCaptchaCheckerConfig),
}

impl CheckerConfig {
    pub async fn build(self, captcha_manager: Arc<Mutex<CaptchaManager>>) -> Result<Checker> {
        Ok(match self {
            CheckerConfig::AuthToken(config) => Checker::from(AuthTokenChecker::new(config)?),
            CheckerConfig::GoogleCaptcha(config) => {
                Checker::from(GoogleCaptchaChecker::new(config)?)
            },
            CheckerConfig::IpBlocklist(config) => Checker::from(IpBlocklistChecker::new(config)?),
            CheckerConfig::MagicHeader(config) => Checker::from(MagicHeaderChecker::new(config)?),
            CheckerConfig::MemoryRatelimit(config) => {
                Checker::from(MemoryRatelimitChecker::new(config))
            },
            CheckerConfig::RedisRatelimit(config) => {
                Checker::from(RedisRatelimitChecker::new(config).await?)
            },
            CheckerConfig::RefererBlocklist(config) => {
                Checker::from(RefererBlocklistChecker::new(config)?)
            },
            CheckerConfig::TapCaptcha(config) => {
                Checker::from(TapCaptchaChecker::new(config, captcha_manager)?)
            },
        })
    }
}

/// This enum has as its variants all possible implementations of CheckerTrait.
#[enum_dispatch(CheckerTrait)]
pub enum Checker {
    AuthTokenChecker,
    GoogleCaptchaChecker,
    IpBlocklistChecker,
    MagicHeaderChecker,
    MemoryRatelimitChecker,
    RedisRatelimitChecker,
    RefererBlocklistChecker,
    TapCaptchaChecker,
}

#[derive(Clone, Debug)]
pub struct CheckerData {
    pub time_request_received_secs: u64,
    pub receiver: AccountAddress,
    pub source_ip: IpAddr,
    pub headers: Arc<HeaderMap>,
}

#[derive(Clone, Debug)]
pub struct CompleteData {
    pub checker_data: CheckerData,
    pub txn_hashes: Vec<String>,
    pub response_is_500: bool,
}
