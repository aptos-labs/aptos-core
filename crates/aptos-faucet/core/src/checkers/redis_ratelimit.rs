// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{CheckerData, CheckerTrait, CompleteData};
use crate::{
    endpoints::{AptosTapError, AptosTapErrorCode, RejectionReason, RejectionReasonCode},
    helpers::{days_since_tap_epoch, get_current_time_secs, seconds_until_next_day},
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use deadpool_redis::{
    redis::{AsyncCommands, ConnectionAddr, ConnectionInfo, RedisConnectionInfo},
    Config, Connection, Pool, Runtime,
};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RedisRatelimitCheckerConfig {
    /// The database address to connect to, not including port,
    /// e.g. db.example.com or 234.121.222.42.
    pub database_address: String,

    /// The port to connect to.
    #[serde(default = "RedisRatelimitCheckerConfig::default_database_port")]
    pub database_port: u16,

    /// The number of the database to use. If it doesn't exist, it will be created (todo verify this)
    #[serde(default = "RedisRatelimitCheckerConfig::default_database_number")]
    pub database_number: i64,

    /// The name of the user to use, if necessary.
    pub database_user: Option<String>,

    /// The password of the given user, if necessary.
    pub database_password: Option<String>,

    /// Max number of requests per IP per day. 500s are not counted, because
    /// they are not the user's fault, but everything else is.
    pub max_requests_per_ip_per_day: u32,
}

impl RedisRatelimitCheckerConfig {
    fn default_database_port() -> u16 {
        6379
    }

    fn default_database_number() -> i64 {
        0
    }

    fn build_connection_info(&self) -> ConnectionInfo {
        ConnectionInfo {
            addr: ConnectionAddr::Tcp(self.database_address.clone(), self.database_port),
            redis: RedisConnectionInfo {
                db: self.database_number,
                username: self.database_user.clone(),
                password: self.database_password.clone(),
            },
        }
    }

    pub fn build_db_pool(&self) -> Result<Pool> {
        let connection_info = self.build_connection_info();
        let cfg = Config {
            connection: Some(connection_info.into()),
            ..Default::default()
        };
        cfg.create_pool(Some(Runtime::Tokio1))
            .context("Failed to build redis connection pool")
    }
}

/// The RedisRatelimitChecker backend uses redis to ratelimit requests to the tap. Unlike
/// the PostgresStorage backend, it does not store full information for each
/// request. Instead, it uses counters to track limits. This is heavily inspired
/// by https://redis.com/redis-best-practices/basic-rate-limiting/.
///
/// If we're not careful, it is possible for people to exceed the intended limit
/// by sending many requests simulatenously. We avoid this problem with this
/// order of operations:
///   1. Read the current value of the limit for source IP.
///   2. If value is greater than limit, reject.
///   3. Otherwise, increment and set TTL if necessary.
///   4. Increment returns the new value. Check if this is greater than the limit also.
///
/// Incrementing the limit is an atomic operation (meaning each client will see
/// value increment, never reading the same value), so steps 1 and 2 are not
/// actually necessary for correctness. Instead, steps 1 and 2 are just an optimization
/// to avoid incrementing the limit unnecessarily if the limit has already been
/// reached. With steps 1 and 2 we end up having more unnecessary reads when
/// they're under their limit vs more unnecessary writes when they're over their
/// limit, but we'll happily take more reads over more writes.
///
/// Note: Previously I made an attempt (d4fbf6db675e9036a967b52bf8d13e1b2566787e) at
/// doing these steps atomically, but it became very unwieldy:
///   1. Start a transaction.
///   2. Increment current value for limit for source IP, set TTL if necessary.
///   3. If value is greater than limit, revert the transaction.
///
/// This second way leaves a small window for someone to slip in multiple requests,
/// therein blowing past the configured limit, but it's a very small window, so
/// we'll worry about it as a followup: https://github.com/aptos-labs/aptos-tap/issues/15.
pub struct RedisRatelimitChecker {
    args: RedisRatelimitCheckerConfig,
    db_pool: Pool,
}

impl RedisRatelimitChecker {
    pub async fn new(args: RedisRatelimitCheckerConfig) -> Result<Self> {
        let db_pool = args.build_db_pool()?;

        // Ensure we can connect.
        db_pool
            .get()
            .await
            .context("Failed to connect to redis on startup")?;

        Ok(Self { args, db_pool })
    }

    pub async fn get_redis_connection(&self) -> Result<Connection, AptosTapError> {
        self.db_pool.get().await.map_err(|e| {
            AptosTapError::new_with_error_code(
                format!("Failed to connect to redis storage: {}", e),
                AptosTapErrorCode::StorageError,
            )
        })
    }

    // Returns the key and the seconds until the next day.
    fn get_key_and_secs_until_next_day(&self, source_ip: &IpAddr) -> (String, u64) {
        let now_secs = get_current_time_secs();
        let seconds_until_next_day = seconds_until_next_day(now_secs);
        let key = format!("ip:{}:{}", source_ip, days_since_tap_epoch(now_secs));
        (key, seconds_until_next_day)
    }

    fn check_limit_value(
        &self,
        data: &CheckerData,
        limit_value: Option<i64>,
        seconds_until_next_day: u64,
    ) -> Option<RejectionReason> {
        if limit_value.unwrap_or(0) > self.args.max_requests_per_ip_per_day as i64 {
            Some(
                RejectionReason::new(
                    format!(
                        "IP {} has reached the maximum allowed number of requests per day: {}",
                        data.source_ip, self.args.max_requests_per_ip_per_day
                    ),
                    RejectionReasonCode::IpUsageLimitExhausted,
                )
                .retry_after(seconds_until_next_day),
            )
        } else {
            None
        }
    }
}

#[async_trait]
impl CheckerTrait for RedisRatelimitChecker {
    async fn check(
        &self,
        data: CheckerData,
        dry_run: bool,
    ) -> Result<Vec<RejectionReason>, AptosTapError> {
        let mut conn = self
            .get_redis_connection()
            .await
            .map_err(|e| AptosTapError::new_with_error_code(e, AptosTapErrorCode::StorageError))?;

        // Generate a key corresponding to this IP address and the current day.
        let (key, seconds_until_next_day) = self.get_key_and_secs_until_next_day(&data.source_ip);

        // Get the value for the key, indicating how many non-500 requests we
        // have serviced for this it today.
        let limit_value: Option<i64> = conn.get(&key).await.map_err(|e| {
            AptosTapError::new_with_error_code(
                format!("Failed to get value for redis key {}: {}", key, e),
                AptosTapErrorCode::StorageError,
            )
        })?;

        // If the limit value is greater than what we allow per day, signal
        // that we should reject this request.
        if let Some(rejection_reason) =
            self.check_limit_value(&data, limit_value, seconds_until_next_day)
        {
            return Ok(vec![rejection_reason]);
        }

        // Atomically increment the counter for the given IP, creating it and
        // setting the expiration time if it doesn't already exist.
        if !dry_run {
            // If the limit value already exists, just increment.
            let incremented_limit_value = match limit_value {
                Some(_) => conn.incr(&key, 1).await.map_err(|e| {
                    AptosTapError::new_with_error_code(
                        format!("Failed to increment redis key {}: {}", key, e),
                        AptosTapErrorCode::StorageError,
                    )
                })?,
                // If the limit value doesn't exist, create it and set the
                // expiration time.
                None => {
                    let (incremented_limit_value,): (i64,) = redis::pipe()
                        .atomic()
                        .incr(&key, 1)
                        // Expire at the end of the day roughly.
                        .expire(&key, seconds_until_next_day as usize)
                        // Only set the expiration if one isn't already set.
                        // Only works with Redis 7 sadly.
                        // .arg("NX")
                        .ignore()
                        .query_async(&mut *conn)
                        .await
                        .map_err(|e| {
                            AptosTapError::new_with_error_code(
                                format!("Failed to increment value for redis key {}: {}", key, e),
                                AptosTapErrorCode::StorageError,
                            )
                        })?;
                    incremented_limit_value
                },
            };

            // Check limit again, to ensure there wasn't a get / set race.
            if let Some(rejection_reason) =
                self.check_limit_value(&data, Some(incremented_limit_value), seconds_until_next_day)
            {
                return Ok(vec![rejection_reason]);
            }
        }

        Ok(vec![])
    }

    /// All we have to do here is decrement the counter if the request was a
    /// failure due to something wrong on our end.
    async fn complete(&self, data: CompleteData) -> Result<(), AptosTapError> {
        if !data.response_is_500 {
            return Ok(());
        }

        let mut conn = self
            .get_redis_connection()
            .await
            .map_err(|e| AptosTapError::new_with_error_code(e, AptosTapErrorCode::StorageError))?;

        // Generate a key corresponding to this IP address and the current day.
        let (key, _) = self.get_key_and_secs_until_next_day(&data.checker_data.source_ip);

        conn.decr(&key, 1).await.map_err(|e| {
            AptosTapError::new_with_error_code(
                format!("Failed to decrement value for redis key {}: {}", key, e),
                AptosTapErrorCode::StorageError,
            )
        })?;
        Ok(())
    }

    fn cost(&self) -> u8 {
        50
    }
}
