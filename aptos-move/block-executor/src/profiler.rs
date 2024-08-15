// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::info;
use aptos_mvhashmap::types::{Incarnation, TxnIndex};
use std::time::{Duration, Instant};

// Total number of callbacks and the time spent, across different (executor) view traits.
pub(crate) struct ViewProfilerState {
    pub(crate) resource_view_stats: (Duration, usize),
    pub(crate) group_view_stats: (Duration, usize),
    pub(crate) module_view_stats: (Duration, usize),
    pub(crate) aggregator_v1_view_stats: (Duration, usize),
    pub(crate) delayed_field_view_stats: (Duration, usize),
}

impl ViewProfilerState {
    pub(crate) fn new() -> Self {
        Self {
            resource_view_stats: (Duration::ZERO, 0),
            group_view_stats: (Duration::ZERO, 0),
            module_view_stats: (Duration::ZERO, 0),
            aggregator_v1_view_stats: (Duration::ZERO, 0),
            delayed_field_view_stats: (Duration::ZERO, 0),
        }
    }

    pub(crate) fn record_resource_view_stat(&mut self, start: Instant) {
        let cur = Instant::now();
        self.resource_view_stats.0 += cur - start;
        self.resource_view_stats.1 += 1;
    }

    pub(crate) fn record_group_view_stat(&mut self, start: Instant) {
        let cur = Instant::now();
        self.group_view_stats.0 += cur - start;
        self.group_view_stats.1 += 1;
    }

    pub(crate) fn record_module_view_stat(&mut self, start: Instant) {
        let cur = Instant::now();
        self.module_view_stats.0 += cur - start;
        self.module_view_stats.1 += 1;
    }

    pub(crate) fn record_aggregator_v1_view_stat(&mut self, start: Instant) {
        let cur = Instant::now();
        self.aggregator_v1_view_stats.0 += cur - start;
        self.aggregator_v1_view_stats.1 += 1;
    }

    pub(crate) fn record_delayed_field_view_stat(&mut self, start: Instant) {
        let cur = Instant::now();
        self.delayed_field_view_stats.0 += cur - start;
        self.delayed_field_view_stats.1 += 1;
    }

    pub(crate) fn log_info(&self, txn_idx: TxnIndex, incarnation: Incarnation, worker_id: usize) {
        info!(
            "TXN = {}, Incarnation = {}, worker_id = {}, callback durations:\
	     ResourceView total time {:?}, num calls {}; \
             GroupView total time {:?}, num calls {};\
             ModuleView total time {:?}, num calls {};\
             DelayedFieldsView total time {:?}, num_calls {};\
             AggregatorV1View total time {:?}, num_calls {}.",
            txn_idx,
            incarnation,
            worker_id,
            self.resource_view_stats.0,
            self.resource_view_stats.1,
            self.group_view_stats.0,
            self.group_view_stats.1,
            self.module_view_stats.0,
            self.module_view_stats.1,
            self.delayed_field_view_stats.0,
            self.delayed_field_view_stats.1,
            self.aggregator_v1_view_stats.0,
            self.aggregator_v1_view_stats.1,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_ge;
    use std::{
        thread::sleep,
        time::{Duration, Instant},
    };

    #[test]
    fn profile_view_callbacks() {
        let mut state = ViewProfilerState::new();
        let start_instant = Instant::now();
        sleep(Duration::from_millis(10));

        state.record_resource_view_stat(start_instant);
        state.record_group_view_stat(start_instant);
        state.record_resource_view_stat(start_instant); // second call.
        state.record_module_view_stat(start_instant);
        state.record_aggregator_v1_view_stat(start_instant);

        assert_eq!(state.delayed_field_view_stats.0.as_millis(), 0);
        assert_eq!(state.delayed_field_view_stats.1, 0);
        state.record_delayed_field_view_stat(start_instant);

        assert_ge!(state.resource_view_stats.0.as_millis(), 10);
        assert_eq!(state.resource_view_stats.1, 2);
        assert_ge!(state.group_view_stats.0.as_millis(), 10);
        assert_eq!(state.group_view_stats.1, 1);
        assert_ge!(state.module_view_stats.0.as_millis(), 10);
        assert_eq!(state.module_view_stats.1, 1);
        assert_ge!(state.aggregator_v1_view_stats.0.as_millis(), 10);
        assert_eq!(state.aggregator_v1_view_stats.1, 1);
        assert_ge!(state.delayed_field_view_stats.0.as_millis(), 10);
        assert_eq!(state.delayed_field_view_stats.1, 1);
    }
}
