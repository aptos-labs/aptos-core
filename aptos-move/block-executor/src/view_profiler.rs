// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]
#![allow(unused_variables)]

use aptos_logger::info;
use aptos_mvhashmap::types::{Incarnation, TxnIndex};
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

// Total number of callbacks and the time spent, across different (executor) view traits.
pub(crate) struct ViewProfiler {
    num_callbacks: Vec<usize>,
    total_duration: Vec<Duration>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum ViewKind {
    ResourceView = 0,
    GroupView = 1,
    ModuleView = 2,
    AggregatorV1View = 3,
    DelayedFieldView = 4,
}

// TODO: use std::mem::variant_count once that is stable.
const NUM_VIEW_KIND: usize = 5;

const VIEW_KINDS: [&str; NUM_VIEW_KIND] = [
    "Resource View",
    "Group View",
    "Module View",
    "Aggregator V1 View",
    "Delayed Field View",
];

pub(crate) struct ViewProfilerGuard<'a> {
    state: &'a RefCell<ViewProfiler>,
    start: Instant,
    kind: ViewKind,
}

impl<'a> ViewProfilerGuard<'a> {
    pub fn new(profiler: &'a RefCell<ViewProfiler>, kind: ViewKind) -> Self {
        Self {
            state: profiler,
            start: Instant::now(),
            kind,
        }
    }
}

impl Drop for ViewProfilerGuard<'_> {
    fn drop(&mut self) {
        let ViewProfilerGuard { state, start, kind } = *self;

        let cur = Instant::now();

        let mut state = state.borrow_mut();
        state.num_callbacks[kind as usize] += 1;
        state.total_duration[kind as usize] += cur - start;
    }
}

impl ViewProfiler {
    pub(crate) fn new() -> Self {
        Self {
            num_callbacks: vec![0; NUM_VIEW_KIND],
            total_duration: vec![Duration::ZERO; NUM_VIEW_KIND],
        }
    }

    pub(crate) fn log_info(&self, txn_idx: TxnIndex, incarnation: Incarnation, worker_id: usize) {
        info!("{:?}", VIEW_KINDS.iter().enumerate().map(|(i, name)| {
            format!(
                "{name} number of callbacks {}, total time {}ms; \n",
                self.num_callbacks[i], self.total_duration[i].as_millis()
            )
        }).fold(format!("TXN = {txn_idx}, Incarnation = {incarnation}, worker_id = {worker_id}, callback durations: \n"),
		|acc, report| acc + &report));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_ge;
    use std::{thread::sleep, time::Duration};
    use test_case::test_case;

    #[test_case(ViewKind::ResourceView)]
    #[test_case(ViewKind::GroupView)]
    #[test_case(ViewKind::ModuleView)]
    #[test_case(ViewKind::AggregatorV1View)]
    #[test_case(ViewKind::DelayedFieldView)]
    fn profile_view_callbacks(kind: ViewKind) {
        let profiler = RefCell::new(ViewProfiler::new());
        let timer = ViewProfilerGuard::new(&profiler, kind);

        let _not_dropped_timer = ViewProfilerGuard::new(&profiler, kind);

        sleep(Duration::from_millis(10));

        drop(timer);

        for i in 0..NUM_VIEW_KIND {
            let profiler = profiler.borrow();
            if i == kind as usize {
                assert_ge!(profiler.total_duration[i].as_millis(), 10);
                assert_eq!(profiler.num_callbacks[i], 1);
            } else {
                assert_eq!(profiler.total_duration[i], Duration::ZERO);
                assert_eq!(profiler.num_callbacks[i], 0);
            }
        }

        {
            let _timer = ViewProfilerGuard::new(&profiler, kind);
            sleep(Duration::from_millis(5));

            let _another_timer = ViewProfilerGuard::new(&profiler, kind);
        }

        for i in 0..NUM_VIEW_KIND {
            let profiler = profiler.borrow();
            if i == kind as usize {
                assert_ge!(profiler.total_duration[i].as_millis(), 5);
                assert_eq!(profiler.num_callbacks[i], 3);
            } else {
                assert_eq!(profiler.total_duration[i], Duration::ZERO);
                assert_eq!(profiler.num_callbacks[i], 0);
            }
        }
    }
}
