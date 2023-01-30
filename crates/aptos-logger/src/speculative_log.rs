// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate as aptos_logger;
/// Infrastucture for logging speculatively, i.e. ability to clear prior logs.
use crate::{debug, error, info, trace, warn, Level};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_log_derive::Schema;
use aptos_state_view::StateViewId;
use aptos_types::transaction::Version;
use arc_swap::ArcSwap;
use crossbeam::utils::CachePadded;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use serde::Serialize;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

#[derive(Default)]
struct BufferedLog {
    entries: Vec<(Level, AdapterLogSchema, String)>, // Log level & the message string
    // critical error count associated with speculative (but not cleared) logs.
    critical_err_cnt: usize,
    // To avoid de-allocations on the critical path, store cleared number.
    skip_cleared: usize,
}

type LOGS = Vec<CachePadded<Mutex<BufferedLog>>>;

// .1 (AtomicUsize type) tracks the total number of speculative critical errors.
// This is done so flush_speculative_logs method can return the number of
// critical errors immediately without traversing over all locks - that is happened
// off the critical path in order to flush the actual messages.
static BUFFERED_LOGS: Lazy<ArcSwap<(LOGS, AtomicUsize)>> =
    Lazy::new(|| ArcSwap::new(Arc::new((LOGS::new(), AtomicUsize::new(0)))));

#[derive(Schema, Clone)]
pub struct AdapterLogSchema {
    name: LogEntryKind,

    // only one of the next 3 `Option`s will be set. Unless it is in testing mode
    // in which case nothing will be set.
    // Those values are coming from `StateView::id()` and the info carried by
    // `StateViewId`

    // StateViewId::BlockExecution - typical transaction execution
    block_id: Option<HashValue>,
    // StateViewId::ChunkExecution - state sync
    first_version: Option<Version>,
    // StateViewId::TransactionValidation - validation
    base_version: Option<Version>,

    // transaction position in the list of transactions in the block,
    // 0 if the transaction is not part of a block (i.e. validation).
    txn_idx: usize,
}

impl AdapterLogSchema {
    pub fn new(view_id: StateViewId, txn_idx: usize) -> Self {
        match view_id {
            StateViewId::BlockExecution { block_id } => Self {
                name: LogEntryKind::Execution,
                block_id: Some(block_id),
                first_version: None,
                base_version: None,
                txn_idx,
            },
            StateViewId::ChunkExecution { first_version } => Self {
                name: LogEntryKind::Execution,
                block_id: None,
                first_version: Some(first_version),
                base_version: None,
                txn_idx,
            },
            StateViewId::TransactionValidation { base_version } => Self {
                name: LogEntryKind::Validation,
                block_id: None,
                first_version: None,
                base_version: Some(base_version),
                txn_idx,
            },
            StateViewId::Miscellaneous => Self {
                name: LogEntryKind::Miscellaneous,
                block_id: None,
                first_version: None,
                base_version: None,
                txn_idx,
            },
        }
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEntryKind {
    Execution,
    Validation,
    Miscellaneous, // usually testing
}

#[macro_export]
macro_rules! speculative_log {
    ($level:expr, $($args:tt)+) => {
        if enabled!($level) {
            speculative_log($level, $($args)+);
        }
    };
}

// Logs directly (no buffering).
fn log_message(level: Level, context: &AdapterLogSchema, message: &String) {
    match level {
        Level::Error => error!(*context, "{}", message),
        Level::Warn => warn!(*context, "{}", message),
        Level::Info => info!(*context, "{}", message),
        Level::Debug => debug!(*context, "{}", message),
        Level::Trace => trace!(*context, "{}", message),
    }
}

/// Adds a message at a specified logging level and given context (that includes txn index)
/// to the ongoing buffer.
/// Note: flush_speculativelogs should be called earlier with the appropriate size to
/// make sure the underlying BUFFERED_LOGS storage is indexed sufficiently.
pub fn speculative_log(level: Level, context: &AdapterLogSchema, message: String, critical: bool) {
    let context = context.clone();

    let txn_idx = context.txn_idx;
    let logs = BUFFERED_LOGS.load();
    let mut log = logs.0[txn_idx].lock();

    // If speculative_log is called before initialization or somehow without resizing the
    // buffer properly, log the message directly and log an error. This is just defensive coding
    // to avoid a panic in such situations (which shouldn't happen).
    if txn_idx >= logs.0.len() {
        // Directly log the message.
        log_message(level, &context, &message);
        error!(
            "speculative_log at idx = {}, but Buffer len = {}",
            txn_idx,
            logs.0.len()
        );
    }

    log.entries.push((level, context, message));
    if critical {
        log.critical_err_cnt += 1;
        logs.1.fetch_add(1, Ordering::SeqCst);
    }
}

/// Clears the buffered log for a transaction without logging anything. Useful e.g. when a
/// speculative transaction execution is aborted by parallel execution (failed validation).
pub fn clear_speculative_log(txn_idx: usize) {
    let logs = BUFFERED_LOGS.load();
    let mut log = logs.0[txn_idx].lock();

    // Logically clear the buffered logs.
    log.skip_cleared = log.entries.len();
    logs.1.fetch_sub(log.critical_err_cnt, Ordering::SeqCst);
    log.critical_err_cnt = 0;
}

/// Useful for e.g. module r/w fallback to sequential execution, as in this case we would
/// like to discard all logs coming from the attempted parallel execution.
pub fn clear_speculative_logs() {
    // TODO: Could parallelize if needed. Currently does logical clear only.
    for i in 0..BUFFERED_LOGS.load().0.len() {
        clear_speculative_log(i);
    }
}

/// Resizes the log buffer, as needed (if not large enough) to the new provided size.
pub fn resize_speculative_logs(num_txns: usize) {
    if num_txns > BUFFERED_LOGS.load().0.len() {
        swap_new_buffer(num_txns);
    }
}

fn swap_new_buffer(num_txns: usize) -> Arc<(LOGS, AtomicUsize)> {
    let swap_to: LOGS = (0..num_txns)
        .map(|_| CachePadded::new(Mutex::new(BufferedLog::default())))
        .collect();
    BUFFERED_LOGS.swap(Arc::new((swap_to, AtomicUsize::new(0))))
}

/// Records all the buffered logs and clears the buffer. The clearing happens
/// synchronously (because the next block may need to start executing and buffering
/// logs), however the flushing happens asynchronously in the global rayon pool.
/// Returns the number of critical errors associated with the flushed logs.
pub fn flush_speculative_logs() -> usize {
    // TODO: if this ends up slow, we can re-use the BufferedLog entries.
    let swapped = swap_new_buffer(BUFFERED_LOGS.load().0.len());

    let ret = swapped.1.load(Ordering::SeqCst);

    rayon::spawn(move || {
        (*swapped.0)
            .par_iter()
            .with_min_len(25)
            .for_each(|log_mutex| {
                let buffered_log = log_mutex.lock();
                for (level, context, message) in
                    buffered_log.entries.iter().skip(buffered_log.skip_cleared)
                {
                    log_message(*level, context, message);
                }
            });
    });

    ret
}

#[cfg(test)]
mod tests {
    use crate::{
        aptos_logger::tests::set_test_logger,
        enabled,
        speculative_log::{
            clear_speculative_log, clear_speculative_logs, flush_speculative_logs,
            resize_speculative_logs, speculative_log, AdapterLogSchema,
        },
        Level,
    };
    use aptos_state_view::StateViewId;
    use claims::assert_err;
    use std::collections::HashSet;

    #[test]
    fn test_speculative_clear() {
        let receiver_mutex = set_test_logger(Level::Debug, true);
        let receiver = receiver_mutex.lock();

        while receiver.try_recv().is_ok() {}

        resize_speculative_logs(2);
        let context_0 = AdapterLogSchema::new(StateViewId::Miscellaneous, 0);
        let context_1 = AdapterLogSchema::new(StateViewId::Miscellaneous, 1);

        // level trace isn't enabled
        speculative_log!(Level::Trace, &context_0, "0/trace: A".to_string(), false);

        speculative_log!(Level::Debug, &context_0, "0/debug: A".to_string(), false);
        speculative_log!(Level::Info, &context_0, "0/info: A".to_string(), false);
        speculative_log!(Level::Warn, &context_0, "0/warn: A".to_string(), false);
        speculative_log!(Level::Error, &context_0, "/0error: A".to_string(), true);
        speculative_log!(Level::Error, &context_1, "1/error: A".to_string(), false);
        speculative_log!(Level::Warn, &context_0, "0/warn: B".to_string(), false);
        // Clear everything above.
        clear_speculative_logs();

        speculative_log!(Level::Warn, &context_0, "0/warn: C".to_string(), true);
        speculative_log!(Level::Error, &context_1, "1/error: B".to_string(), true); // Expected
        speculative_log!(Level::Error, &context_0, "0/error: B".to_string(), false);
        speculative_log!(Level::Error, &context_0, "0/error: C".to_string(), true);
        speculative_log!(Level::Info, &context_0, "0/warn: B".to_string(), false);
        // Clear only logs from thread idx = 0 (context_0).
        clear_speculative_log(0);
        speculative_log!(Level::Warn, &context_1, "1/warn: A".to_string(), false); // Expected
        speculative_log!(Level::Trace, &context_0, "0/trace: B".to_string(), true); // not enabled
        speculative_log!(Level::Trace, &context_0, "1/trace: A".to_string(), false); // not enabled
        speculative_log!(Level::Warn, &context_1, "1/error: C".to_string(), true); // Expected
        speculative_log!(Level::Debug, &context_0, "0/debug: B".to_string(), false); // Expected

        assert_err!(receiver.try_recv());

        assert_eq!(flush_speculative_logs(), 2);

        // We expect 4 messages.
        let expected = vec![
            "1/error: B".to_string(),
            "1/warn: A".to_string(),
            "1/error: C".to_string(),
            "0/debug: B".to_string(),
        ];
        let mut expected_set: HashSet<String> = expected.into_iter().collect();

        for _ in 0..4 {
            let m = receiver.recv();
            assert!(expected_set.remove(m.expect("expected a message").message().unwrap()));
        }
        assert_err!(receiver.try_recv());
    }
}
