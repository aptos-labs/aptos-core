// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_logger::info;
use aptos_mvhashmap::types::{Incarnation, TxnIndex};
use std::{
    cell::RefCell,
    time::{Duration, Instant},
};

// Total number of callbacks and the time spent, across different (executor) view traits.
pub(crate) struct ViewProfilerState {
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

pub(crate) struct ViewProfilerTimer<'a> {
    state: &'a RefCell<ViewProfilerState>,
    start: Instant,
    kind: ViewKind,
}

impl<'a> ViewProfilerTimer<'a> {
    pub fn new(
        maybe_profiler_state: &'a Option<RefCell<ViewProfilerState>>,
        kind: ViewKind,
    ) -> Option<Self> {
        maybe_profiler_state.as_ref().map(|state| Self {
            state,
            start: Instant::now(),
            kind,
        })
    }
}

impl Drop for ViewProfilerTimer<'_> {
    fn drop(&mut self) {
        let ViewProfilerTimer { state, start, kind } = *self;

        let cur = Instant::now();

        let mut state = state.borrow_mut();
        state.num_callbacks[kind as usize] += 1;
        state.total_duration[kind as usize] += cur - start;
    }
}

impl ViewProfilerState {
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
        let state = Some(RefCell::new(ViewProfilerState::new()));
        let timer = ViewProfilerTimer::new(&state, kind);

        let empty_timer = ViewProfilerTimer::new(&None, kind);
        let empty_group_timer = ViewProfilerTimer::new(&None, ViewKind::GroupView);

        let _not_dropped_timer = ViewProfilerTimer::new(&state, kind);
        let _not_dropped_empty_timer = ViewProfilerTimer::new(&None, kind);

        sleep(Duration::from_millis(10));

        drop(timer);
        drop(empty_timer);

        for i in 0..NUM_VIEW_KIND {
            let state = state.as_ref().unwrap().borrow();
            if i == kind as usize {
                assert_ge!(state.total_duration[i].as_millis(), 10);
                assert_eq!(state.num_callbacks[i], 1);
            } else {
                assert_eq!(state.total_duration[i], Duration::ZERO);
                assert_eq!(state.num_callbacks[i], 0);
            }
        }

        drop(empty_group_timer);

        {
            let _timer = ViewProfilerTimer::new(&state, kind);
            sleep(Duration::from_millis(5));

            let _another_timer = ViewProfilerTimer::new(&state, kind);
        }

        for i in 0..NUM_VIEW_KIND {
            let state = state.as_ref().unwrap().borrow();
            if i == kind as usize {
                assert_ge!(state.total_duration[i].as_millis(), 5);
                assert_eq!(state.num_callbacks[i], 3);
            } else {
                assert_eq!(state.total_duration[i], Duration::ZERO);
                assert_eq!(state.num_callbacks[i], 0);
            }
        }
    }
}
