// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::types::MempoolLatencySummary;
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

pub struct MempoolLatencyStatsTracking {
    stats: VecDeque<MempoolLatencySummary>,
    cur_time_start: Instant,
    num_intervals: usize,
    time_per_interval: Duration,
}

impl MempoolLatencyStatsTracking {
    pub fn new(num_intervals: usize, time_per_interval: Duration) -> Self {
        Self {
            stats: VecDeque::from(vec![MempoolLatencySummary::empty()]),
            cur_time_start: Instant::now(),
            num_intervals,
            time_per_interval,
        }
    }

    pub fn get_latency_summary(&self, target_samples: usize) -> MempoolLatencySummary {
        let mut total = MempoolLatencySummary::empty();
        for stat in self.stats.iter().rev() {
            total.aggregate(stat);
            if total.count >= target_samples {
                break;
            }
        }

        total
    }

    pub fn check_rollover(&mut self) {
        if self.cur_time_start.elapsed() > self.time_per_interval {
            if self.stats.len() >= self.num_intervals {
                self.stats.pop_front();
            }
            self.stats.push_back(MempoolLatencySummary::empty())
        }
    }

    pub fn get_current_mut(&mut self) -> &mut MempoolLatencySummary {
        self.stats.back_mut().unwrap()
    }
}
