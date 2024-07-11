// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_mempool_notifications::CommittedTransaction;
use aptos_types::transaction::use_case::UseCaseKey;
use std::collections::{HashMap, HashSet, VecDeque};

pub(crate) struct UseCaseHistory {
    window_size: usize,
    num_top_to_track: usize,
    recent: VecDeque<HashMap<UseCaseKey, usize>>,
}

impl UseCaseHistory {
    pub(crate) fn new(window_size: usize, num_top_to_track: usize) -> Self {
        Self {
            window_size,
            num_top_to_track,
            recent: VecDeque::with_capacity(window_size + 1),
        }
    }

    pub(crate) fn update_usecases_and_get_tracking_set(
        &mut self,
        transactions: &[CommittedTransaction],
    ) -> HashSet<UseCaseKey> {
        let mut count_by_usecase = HashMap::new();
        for transaction in transactions {
            *count_by_usecase
                .entry(transaction.use_case.clone())
                .or_insert(0) += 1;
        }

        self.recent.push_back(count_by_usecase);
        while self.recent.len() > self.window_size {
            self.recent.pop_front();
        }

        let mut total = HashMap::new();
        for group in &self.recent {
            for (use_case, count) in group {
                if use_case != &UseCaseKey::Platform && use_case != &UseCaseKey::Others {
                    *total.entry(use_case.clone()).or_insert(0) += count;
                }
            }
        }

        let mut result = HashSet::new();
        result.insert(UseCaseKey::Platform);
        result.insert(UseCaseKey::Others);

        let mut sorted = total.into_iter().collect::<Vec<_>>();
        sorted.sort_by_key(|(_, count)| *count);

        for _ in 0..self.num_top_to_track {
            if let Some((use_case, _)) = sorted.pop() {
                result.insert(use_case);
            }
        }

        result
    }
}
