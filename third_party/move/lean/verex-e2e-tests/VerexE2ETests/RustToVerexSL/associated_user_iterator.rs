// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub struct Countdown {
    remaining: u32,
}

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        Some(self.remaining)
    }
}

pub fn total(countdown: Countdown) -> u32 {
    let mut total = 0;
    for value in countdown {
        total += value;
    }
    total
}

pub fn final_value(countdown: Countdown) -> Option<u32> {
    countdown.last()
}
