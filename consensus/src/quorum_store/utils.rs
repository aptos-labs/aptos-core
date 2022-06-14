// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::HashValue;
use chrono::Utc;
use std::collections::VecDeque;

pub(crate) struct DigestTimeouts {
    timeouts: VecDeque<(i64, HashValue)>,
}

impl DigestTimeouts {
    pub(crate) fn new() -> Self {
        Self {
            timeouts: VecDeque::new(),
        }
    }

    pub(crate) fn add_digest(&mut self, digest: HashValue, timeout: usize) {
        let expiry = Utc::now().naive_utc().timestamp_millis() + timeout as i64;
        self.timeouts.push_back((expiry, digest));
    }

    pub(crate) fn expire(&mut self) -> Vec<HashValue> {
        let cur_time = chrono::Utc::now().naive_utc().timestamp_millis();
        let num_expired = self
            .timeouts
            .iter()
            .take_while(|(expiration_time, _)| cur_time >= *expiration_time)
            .count();

        self.timeouts
            .drain(0..num_expired)
            .map(|(_, h)| h)
            .collect()
    }
}
