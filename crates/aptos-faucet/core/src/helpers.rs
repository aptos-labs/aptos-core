// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_sdk::{crypto::HashValue, types::transaction::SignedTransaction};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn transaction_hashes(transactions: &[&SignedTransaction]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for transaction in transactions.iter() {
        // This is clearly gross. If we commit to returning only txn hashes
        // we can simplify this: https://github.com/aptos-labs/aptos-tap/issues/20.
        let c = transaction.to_owned().to_owned();
        let hash: HashValue = c.submitted_txn_hash();
        out.push(hash.to_string());
    }
    out
}

pub fn get_current_time_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time has gone backwards???")
        .as_secs()
}

/// This unixtime is 12:01am PDT on 2021-09-25. See the docstring for
/// RedisRatelimitChecker for more information on how we use this value.
/// We also use this in MemoryRatelimitChecker in a similar way.
pub const TAP_EPOCH_SECS: u64 = 1664089260;

/// Get the number of days since the tap epoch. See the docstring for
/// RedisRatelimitChecker.
pub fn days_since_tap_epoch(current_time_secs: u64) -> u64 {
    (current_time_secs - TAP_EPOCH_SECS) / 86400
}

pub fn seconds_until_next_day(current_time_secs: u64) -> u64 {
    let seconds_since_midnight = current_time_secs % 86400;
    86400 - seconds_since_midnight
}
