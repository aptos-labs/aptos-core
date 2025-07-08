// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{CheckerData, CheckerTrait, CompleteData};
use crate::{
    endpoints::{AptosTapError, RejectionReason, RejectionReasonCode},
    helpers::{days_since_tap_epoch, get_current_time_secs},
};
use async_trait::async_trait;
use lru::LruCache;
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, num::NonZeroUsize, sync::atomic::AtomicU64};
use tokio::sync::Mutex;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MemoryRatelimitCheckerConfig {
    pub max_requests_per_day: u32,

    #[serde(default = "MemoryRatelimitCheckerConfig::default_max_entries_in_map")]
    pub max_entries_in_map: NonZeroUsize,
}

impl MemoryRatelimitCheckerConfig {
    fn default_max_entries_in_map() -> NonZeroUsize {
        NonZeroUsize::new(1000000).unwrap()
    }
}

/// Simple in memory storage that rejects if we've ever seen a request from an
/// IP that has succeeded. This does not support JWT-based ratelimiting.
pub struct MemoryRatelimitChecker {
    pub max_requests_per_day: u32,

    /// Map of IP to how many requests they've submitted today (where the
    /// response wasn't a 500). To avoid OOMing the server, we set a limit
    /// on how many entries we have in the table.
    pub ip_to_requests_today: Mutex<LruCache<IpAddr, u32>>,

    /// Used for tracking daily ratelimit. See the comment in RedisRatelimitChecker
    /// for more information on how we track daily limits.
    pub current_day: AtomicU64,
}

impl MemoryRatelimitChecker {
    pub fn new(args: MemoryRatelimitCheckerConfig) -> Self {
        Self {
            max_requests_per_day: args.max_requests_per_day,
            ip_to_requests_today: Mutex::new(LruCache::new(
                NonZeroUsize::new(args.max_entries_in_map).expect("LRU capacity must be non zero."),
            )),
            current_day: AtomicU64::new(days_since_tap_epoch(get_current_time_secs())),
        }
    }

    async fn clear_if_new_day(&self) {
        if days_since_tap_epoch(get_current_time_secs())
            > self.current_day.load(std::sync::atomic::Ordering::Relaxed)
        {
            self.current_day.store(
                days_since_tap_epoch(get_current_time_secs()),
                std::sync::atomic::Ordering::Relaxed,
            );
            self.ip_to_requests_today.lock().await.clear();
        }
    }
}

#[async_trait]
impl CheckerTrait for MemoryRatelimitChecker {
    async fn check(
        &self,
        data: CheckerData,
        dry_run: bool,
    ) -> Result<Vec<RejectionReason>, AptosTapError> {
        self.clear_if_new_day().await;

        let mut ip_to_requests_today = self.ip_to_requests_today.lock().await;

        let requests_today = ip_to_requests_today.get_or_insert_mut(data.source_ip, || 1);
        if *requests_today >= self.max_requests_per_day {
            return Ok(vec![RejectionReason::new(
                format!(
                    "IP {} has exceeded the daily limit of {} requests",
                    data.source_ip, self.max_requests_per_day
                ),
                RejectionReasonCode::UsageLimitExhausted,
            )]);
        } else if !dry_run {
            *requests_today += 1;
        }

        Ok(vec![])
    }

    async fn complete(&self, data: CompleteData) -> Result<(), AptosTapError> {
        if data.response_is_500 {
            *self
                .ip_to_requests_today
                .lock()
                .await
                .get_or_insert_mut(data.checker_data.source_ip, || 1) -= 1;
        }
        Ok(())
    }

    fn cost(&self) -> u8 {
        20
    }
}
