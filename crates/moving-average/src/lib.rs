// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use std::collections::VecDeque;

// TPS data
pub struct MovingAverage {
    window_millis: u64,
    // (timestamp_millis, value)
    values: VecDeque<(u64, u64)>,
    sum: u64,
}

impl MovingAverage {
    pub fn new(window_millis: u64) -> Self {
        Self {
            window_millis,
            values: VecDeque::new(),
            sum: 0,
        }
    }

    pub fn tick_now(&mut self, value: u64) {
        let now = chrono::Utc::now().naive_utc().timestamp_millis() as u64;
        self.tick(now, value);
    }

    pub fn tick(&mut self, timestamp_millis: u64, value: u64) -> f64 {
        self.values.push_back((timestamp_millis, value));
        self.sum += value;
        loop {
            match self.values.front() {
                None => break,
                Some((ts, val)) => {
                    if timestamp_millis - ts > self.window_millis {
                        self.sum -= val;
                        self.values.pop_front();
                    } else {
                        break;
                    }
                },
            }
        }
        self.avg()
    }

    pub fn avg(&self) -> f64 {
        if self.values.len() < 2 {
            0.0
        } else {
            let elapsed = self.values.back().unwrap().0 - self.values.front().unwrap().0;
            self.sum as f64 / elapsed as f64
        }
    }

    pub fn sum(&self) -> u64 {
        self.sum
    }
}
