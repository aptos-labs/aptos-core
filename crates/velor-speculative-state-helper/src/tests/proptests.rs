// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{SpeculativeCounter, SpeculativeEvent, SpeculativeEvents};
use claims::{assert_err, assert_ok};
use crossbeam::utils::CachePadded;
use parking_lot::RwLock;
use proptest::{collection::vec, prelude::*};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Weak,
};

// Internally uses speculative counter, i.e. for the event itself to record the side effect.
#[derive(Clone)]
struct CounterEvent {
    shared_counter: Weak<SpeculativeCounter>,
    shared_task_cnt: Arc<AtomicUsize>,
    idx: usize,
    delta: usize,
}

impl CounterEvent {
    fn new(
        shared_counter: Weak<SpeculativeCounter>,
        shared_task_cnt: Arc<AtomicUsize>,
        idx: usize,
        delta: usize,
    ) -> Self {
        Self {
            shared_counter,
            shared_task_cnt,
            idx,
            delta,
        }
    }
}

impl SpeculativeEvent for CounterEvent {
    fn dispatch(self) {
        assert_ok!(self
            .shared_counter
            .upgrade()
            .expect("Counter must exist")
            .fetch_add(self.idx, self.delta));
        self.shared_task_cnt.fetch_add(1, Ordering::Relaxed);
    }
}

// The work that's generated for testing purposes for workers / threads consists of these
// operators, e.g. checking that a specific index is out of bounds of Speculative storage,
// or invoking addition on the speculative counter (event does addition via SpeculativeEvents
// interface). Clear operator clears speculative events storage or counter at a given index.
// Finally, prior number of clears is provided and used as a barrier so the results can be
// deterministic despite concurrency in the multi-threaded testing.
#[derive(Clone)]
enum Operator {
    OutOfBounds(usize),         // txn_idx
    Clear(usize, usize),        // txn_idx, prior number of clears
    Add(usize, usize, usize),   // txn_idx, delta, prior number of clears
    Event(CounterEvent, usize), // addition event, prior number of clears
}

// Returns the vector of operations for each worker, and the baseline output (counter values)
fn prepare_work(
    num_workers: usize,
    num_counters: usize,
    counter_ops: Vec<(usize, usize)>,
    maybe_arc_event_counter: Option<(Weak<SpeculativeCounter>, Arc<AtomicUsize>)>,
) -> (Vec<Vec<Operator>>, Vec<usize>, Vec<usize>) {
    assert!(counter_ops.len() == 800);

    let mut num_cleared = vec![0; num_counters];
    let mut counter_values = vec![0; num_counters];

    // Pre-process indices and deltas and put them into worker tasks.
    let mut worker_tasks: Vec<Vec<Operator>> = (0..4).map(|_| Vec::with_capacity(200)).collect();

    for (i, (mut idx, mut delta)) in counter_ops.iter().enumerate() {
        // Group based on modulo (num_counters + 1). Last group is for testing out of bounds.
        let modulo = idx % (num_counters + 1);
        if modulo != num_counters {
            idx = modulo;
        }
        // delta = 0 is for clearing.
        delta %= 7;

        // baseline computation and keeping track of prior number of clears for determinism.
        let op = if idx < num_counters {
            if delta == 0 {
                // Simulate the clear.
                num_cleared[idx] += 1;
                counter_values[idx] = 0;
                Operator::Clear(idx, num_cleared[idx] - 1)
            } else {
                counter_values[idx] += delta;
                if let Some((arc_event_counter, arc_event_done_cnt)) = &maybe_arc_event_counter {
                    Operator::Event(
                        CounterEvent::new(
                            arc_event_counter.clone(),
                            arc_event_done_cnt.clone(),
                            idx,
                            delta,
                        ),
                        num_cleared[idx],
                    )
                } else {
                    Operator::Add(idx, delta, num_cleared[idx])
                }
            }
        } else {
            Operator::OutOfBounds(idx)
        };

        worker_tasks[i % num_workers].push(op);
    }

    (worker_tasks, counter_values, num_cleared)
}

fn test_counter(counter_ops: Vec<(usize, usize)>, test_total: bool) {
    let num_workers = 4;
    let num_counters = 10;

    let (worker_tasks, final_counts, _) =
        prepare_work(num_workers, num_counters, counter_ops, None);

    // Create speculative counters.
    let spec_cnt = SpeculativeCounter::new(num_counters);

    // Check special case for out of bounds
    assert_err!(spec_cnt.fetch_add(num_counters, 1));
    assert_err!(spec_cnt.set_counter_to_zero(num_counters));

    let idx_gen = AtomicUsize::new(0);
    let clear_barriers: Vec<CachePadded<RwLock<usize>>> = (0..num_counters)
        .map(|_| (CachePadded::new(RwLock::new(0))))
        .collect();
    rayon::scope(|s| {
        for _ in 0..num_workers {
            s.spawn(|_| {
                let worker_idx = idx_gen.fetch_add(1, Ordering::Relaxed);
                for op in &worker_tasks[worker_idx] {
                    match op {
                        Operator::OutOfBounds(idx) => {
                            assert_err!(spec_cnt.set_counter_to_zero(*idx));
                            assert_err!(spec_cnt.fetch_add(*idx, 1));
                        },
                        Operator::Clear(idx, clear_barrier) => {
                            // Make sure we don't clear out of order
                            while *clear_barriers[*idx].read() != *clear_barrier {}

                            let mut clear_cnt = clear_barriers[*idx].write();
                            assert_eq!(*clear_cnt, *clear_barrier);
                            // Clear it while holding the lock and increment the barrier.
                            assert_ok!(spec_cnt.set_counter_to_zero(*idx));
                            *clear_cnt += 1;
                        },
                        Operator::Add(idx, delta, clear_barrier) => {
                            // Make sure we don't add before the corresponding clear
                            while *clear_barriers[*idx].read() < *clear_barrier {}

                            // Make sure we don't add after the corresponding clear.
                            let clear_cnt = clear_barriers[*idx].read();
                            if *clear_cnt == *clear_barrier {
                                // Holding the lock, so number of clears can't change.
                                assert_ok!(spec_cnt.fetch_add(*idx, *delta));
                            }
                        },
                        Operator::Event(..) => unreachable!("Not testing events"),
                    }
                }
            });
        }
    });
    if test_total {
        assert_eq!(spec_cnt.take_total(), final_counts.iter().sum());
    } else {
        assert_eq!(spec_cnt.take_counts(), final_counts);
    }
}

fn test_events(counter_ops: Vec<(usize, usize)>) {
    let num_workers = 4;
    let num_counters = 10;

    // Create speculative events.
    let spec_events = SpeculativeEvents::<CounterEvent>::new(num_counters);
    let arc_spec_cnt = Arc::new(SpeculativeCounter::new(num_counters));
    let arc_event_done_cnt = Arc::new(AtomicUsize::new(0));

    let (worker_tasks, final_counts, clear_counts) = prepare_work(
        num_workers,
        num_counters,
        counter_ops,
        Some((Arc::downgrade(&arc_spec_cnt), arc_event_done_cnt.clone())),
    );

    // Check special case for out of bounds
    assert_err!(spec_events.clear_txn_events(num_counters));

    let idx_gen = AtomicUsize::new(0);
    let worker_done_cnt = AtomicUsize::new(0);
    let clear_barriers: Vec<CachePadded<RwLock<usize>>> = (0..num_counters)
        .map(|_| (CachePadded::new(RwLock::new(0))))
        .collect();
    // This is the number of events we expect to be dispatched at the end.
    let recorded_final_event_cnt = AtomicUsize::new(0);
    rayon::scope(|s| {
        for _ in 0..num_workers {
            s.spawn(|_| {
                let worker_idx = idx_gen.fetch_add(1, Ordering::Relaxed);
                for op in &worker_tasks[worker_idx] {
                    match op {
                        Operator::OutOfBounds(idx) => {
                            assert_err!(spec_events.clear_txn_events(*idx));
                        },
                        Operator::Clear(idx, clear_barrier) => {
                            // Make sure we don't clear out of order
                            while *clear_barriers[*idx].read() != *clear_barrier {}

                            let mut clear_cnt = clear_barriers[*idx].write();
                            assert_eq!(*clear_cnt, *clear_barrier);
                            // Clear it while holding the lock and increment the barrier.
                            assert_ok!(spec_events.clear_txn_events(*idx));
                            *clear_cnt += 1;
                        },
                        Operator::Event(event, clear_barrier) => {
                            let idx = event.idx;
                            // Make sure we don't add before the corresponding clear
                            while *clear_barriers[idx].read() < *clear_barrier {}

                            // Make sure we don't add after the corresponding clear.
                            let clear_cnt = clear_barriers[idx].read();
                            if *clear_cnt == *clear_barrier {
                                // Holding the lock, so number of clears can't change.
                                assert_ok!(spec_events.record(idx, event.clone()));

                                if *clear_cnt == clear_counts[idx] {
                                    // This was after the final clear
                                    recorded_final_event_cnt.fetch_add(1, Ordering::Relaxed);
                                }
                            }
                        },
                        Operator::Add(..) => {
                            unreachable!("Not testing counter")
                        },
                    }
                }
                worker_done_cnt.fetch_add(1, Ordering::Relaxed);
            });
        }
    });

    while worker_done_cnt.load(Ordering::Relaxed) != 4 {}

    spec_events.flush(num_counters);
    // Need the number of recorded events after last clear.
    let expected_dispatched_events = recorded_final_event_cnt.load(Ordering::Relaxed);
    while arc_event_done_cnt.load(Ordering::Relaxed) != expected_dispatched_events {}

    // Wait until all Arc's in the tasks are cleared.
    while Arc::strong_count(&arc_spec_cnt) > 1 {}

    assert_eq!(
        Arc::try_unwrap(arc_spec_cnt)
            .expect("Must be single owner")
            .take_counts(),
        final_counts
    );
}

proptest! {
   #[test]
    fn concurrent_counter_total_proptest(
        counter_ops in vec((any::<usize>(), any::<usize>()), 800),
    ) {
        // counter ops determines the index & increment to the speculative counter. We also
        // test SpeculativeEvents by implementing counter updates via events.
        test_counter(counter_ops, true);
    }

    #[test]
    fn concurrent_counter_values_proptest(
        counter_ops in vec((any::<usize>(), any::<usize>()), 800),
    ) {
        // counter ops determines the index & increment to the speculative counter. We also
        // test SpeculativeEvents by implementing counter updates via events.
        test_counter(counter_ops, false);
    }

    #[test]
    fn concurrent_events_proptest(
        counter_ops in vec((any::<usize>(), any::<usize>()), 800),
    ) {
        // counter ops determines the index & increment to the speculative counter. We also
        // test SpeculativeEvents by implementing counter updates via events.
        test_events(counter_ops);
    }
}
