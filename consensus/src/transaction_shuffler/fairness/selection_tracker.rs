// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::fairness::TxnIdx;

pub struct SelectionTracker {
    selected_markers: Vec<bool>,
    cur_idx: usize,
}

impl SelectionTracker {
    pub fn new(num_txns: usize) -> Self {
        Self {
            selected_markers: vec![false; num_txns],
            cur_idx: 0,
        }
    }

    pub fn next_unselected(&mut self) -> Option<TxnIdx> {
        while self.cur_idx < self.selected_markers.len() {
            let idx = self.cur_idx;
            self.cur_idx += 1;

            if !self.is_selected(idx) {
                return Some(idx);
            }
        }
        None
    }

    pub fn new_pass(&mut self) {
        self.cur_idx = 0
    }

    pub fn mark_selected(&mut self, idx: TxnIdx) {
        assert!(!self.selected_markers[idx]);
        self.selected_markers[idx] = true;
    }

    fn is_selected(&self, idx: TxnIdx) -> bool {
        self.selected_markers[idx]
    }
}
