// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This crate is designed to help with tracking events and state that might be speculative in
//! nature due to speculative execution of transactions (e.g. by BlockSTM parallel executor, but
//! also in any other context). The idea is that a transaction may be speculative executed, but
//! then the speculative execution might be invalidated and discarded, possibly triggering
//! another re-execution. In this case, it is convenient to buffer certain state and events and
//! also discard them, while having a way to flush in case the execution is actually finalized.
//! All components here can (and are intended to) be used in a concurrent fashion.
//!
//! An important feature is that flush for events happens asynchronously (and in parallel via
//! rayon global pool), and a number of optimizations ensures that the operations on the critical
//! path do not introduce too much overhead (e.g. clearing, initialization).
//!
//! As far as the state is concerned, the crate currently just implements a SpeculativeCounter.
//! In the future, we could easily implement a structure similar to SpeculativeEvents to
//! keep track of any speculative state, where a trait could allow updating it (running state).
//!
//! An example of using the crate for speculative logging can be founds in tests/logging.rs

// Suppress incorrect warning, as without mut the take_counts method stops compiling.
#![allow(unused_mut)]

use aptos_infallible::Mutex;
use crossbeam::utils::CachePadded;
use rayon::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(test)]
mod tests;

const EVENT_DISPATCH_BATCH_SIZE: usize = 25;

/// A trait for a speculative event, only requiring a dispatch() method that destroys the
/// event and manifests the desired behavior as a side effect (returning nothing). This occurs
/// if and when a speculative event actually gets finalized.
pub trait SpeculativeEvent {
    fn dispatch(self);
}

// A type alias for an event storage container.
type EventStore<T> = Vec<CachePadded<Mutex<Vec<T>>>>;

/// A struct that stores speculative events for transactions, indexed by an usize. Allows
/// clearing the speculative events for a specific transaction or all transactions, and flushing
/// the (non-cleared) events when the executions are finalized. The underlying storage must be
/// sized to fit the indices of transactions, set by the new(num_txns) call for initialization.
pub struct SpeculativeEvents<E: Send> {
    events: EventStore<E>,
}

impl<E: Send + SpeculativeEvent + 'static> SpeculativeEvents<E> {
    // Returns a ref to the current storage of all events if its length is sufficiently large.
    fn events_with_checked_length(&self, min_length: usize) -> anyhow::Result<&EventStore<E>> {
        let len = self.events.len();
        if len < min_length {
            anyhow::bail!(
                "speculative events storage len = {} < required {} (was not sized appropriately)",
                len,
                min_length
            );
        }
        Ok(&self.events)
    }

    /// Create a new storage for recording speculative events by transactions indexed 0..num_txns.
    pub fn new(num_txns: usize) -> Self {
        Self {
            events: (0..num_txns)
                .map(|_| CachePadded::new(Mutex::new(Vec::new())))
                .collect(),
        }
    }

    /// Error means that the underlying speculative event storage was not properly sized to store
    /// an event for the provided transaction index.
    pub fn record(&self, txn_idx: usize, event: E) -> anyhow::Result<()> {
        let events = self.events_with_checked_length(txn_idx + 1)?;

        // TODO: check the common size and the number of elements, as it may be worthwhile
        // to override the capacity defaults of a Vec.
        events[txn_idx].lock().push(event);
        Ok(())
    }

    /// Clears events recorded so far for a given transaction.
    pub fn clear_txn_events(&self, txn_idx: usize) -> anyhow::Result<()> {
        let events = self.events_with_checked_length(txn_idx + 1)?;
        events[txn_idx].lock().clear();
        Ok(())
    }

    /// Clears all events.
    pub fn clear_all_events(&self) {
        // TODO: Parallelize if needed.
        for event in &self.events {
            event.lock().clear();
        }
    }

    /// Flush the first num_to_flush stored events asynchronously by spawning global rayon threads.
    pub fn flush(mut self, num_to_flush: usize) {
        let num_to_flush = num_to_flush.min(self.events.len());
        let to_flush = self.events.drain(..num_to_flush).collect::<Vec<_>>();
        rayon::spawn(move || {
            to_flush
                .into_par_iter()
                .with_min_len(EVENT_DISPATCH_BATCH_SIZE)
                .for_each(|m| {
                    for event in m.into_inner().into_inner() {
                        event.dispatch();
                    }
                });
        });
    }
}

// A type alias for event storage container. Note: can generalize to other Atomics via traits.
#[derive(Debug)]
struct CounterStore {
    counts: Vec<CachePadded<AtomicUsize>>,
    total: AtomicUsize,
}

impl CounterStore {
    fn new(len: usize) -> Self {
        Self {
            counts: (0..len)
                .map(|_| CachePadded::new(AtomicUsize::new(0)))
                .collect(),
            total: AtomicUsize::new(0),
        }
    }

    fn take_counts(mut self) -> Vec<usize> {
        self.counts
            .into_iter()
            .map(|c| c.load(Ordering::Acquire))
            .collect()
    }
}

/// A struct that stores speculative counts per transaction, allowing to clear the current count
/// for a transaction (e.g. if the transaction aborted). Clearing counts for all transactions
/// is not supported as a new SpeculativeCounter can instead be created. The take_total and
/// take_counts methods consume the SpeculativeCounter as they are intended to be called after
/// speculative executions in the quiescent period. The implementation is optimized for tracking
/// the total value without having to sum up individual counts, adding some overhead for when
/// it is needed to track the counts per individual transactions.
#[derive(Debug)]
pub struct SpeculativeCounter {
    count_store: CounterStore,
}

impl SpeculativeCounter {
    // Returns a ref to the current storage of all counters if its length is sufficiently large.
    fn store_with_checked_length(&self, min_length: usize) -> anyhow::Result<&CounterStore> {
        let len = self.count_store.counts.len();
        if len < min_length {
            anyhow::bail!(
                "speculative counters storage len = {} < required {} (was not sized appropriately)",
                len,
                min_length
            );
        }
        Ok(&self.count_store)
    }

    /// Create a new storage for speculative counting by transactions indexed 0..num_txns.
    pub fn new(num_txns: usize) -> Self {
        Self {
            count_store: CounterStore::new(num_txns),
        }
    }

    /// Error means that the underlying speculative storage was not properly sized to count
    /// for the provided transaction index. The returned usize value in Ok() is the current
    /// speculative count (ignoring all the previously cleared counts).
    pub fn fetch_add(&self, txn_idx: usize, delta: usize) -> anyhow::Result<usize> {
        let store = self.store_with_checked_length(txn_idx + 1)?;

        // We are handling the total counter non-atomically, but it is fine because it
        // can only be consumed by take.
        store.total.fetch_add(delta, Ordering::Relaxed);

        Ok(store.counts[txn_idx].fetch_add(delta, Ordering::SeqCst))
    }

    /// Sets the counter for a given transaction to zero (clearing the speculative count).
    pub fn set_counter_to_zero(&self, txn_idx: usize) -> anyhow::Result<()> {
        let store = self.store_with_checked_length(txn_idx + 1)?;

        let prev_cnt = store.counts[txn_idx].swap(0, Ordering::SeqCst);
        let prev_total = store.total.fetch_sub(prev_cnt, Ordering::SeqCst);
        assert!(
            prev_total >= prev_cnt,
            "Total count can't be smaller than transaction counter value"
        );

        Ok(())
    }

    /// Consume self and return the total counter (sum across per-txn counters) value.
    pub fn take_total(self) -> usize {
        self.count_store.total.load(Ordering::SeqCst)
    }

    /// Consume self and return the final values of all counters.
    pub fn take_counts(mut self) -> Vec<usize> {
        self.count_store.take_counts()
    }
}
