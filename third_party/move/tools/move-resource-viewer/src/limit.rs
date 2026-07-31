// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::vm_status::StatusCode;

// Default budget of live heap per query.
pub const DEFAULT_MAX_ANNOTATION_BYTES: usize = 100_000_000_000;

/// Observes real heap growth during a single synchronous annotation request and aborts when it
/// exceeds `budget`. `read_live` returns the calling thread's current live bytes (see
/// `aptos_jemalloc::current_live_bytes`); the difference from `start_live` is how much this
/// request has allocated. We measure real heap growth rather than estimating per-node cost.
///
/// Checks run *after* allocation, so the meter bounds *cumulative* growth across the annotation
/// tree, not the peak of any single `simple_deserialize`. The process-wide resident gate
/// (`memory_admission_cap_bytes`) is the backstop for one oversized allocation that blows the
/// budget before the next check fires.
pub struct Meter {
    budget: usize,
    start_live: i64,
    read_live: fn() -> i64,
}

impl Meter {
    pub fn new(budget: usize, read_live: fn() -> i64) -> Self {
        let start_live = read_live();
        Self {
            budget,
            start_live,
            read_live,
        }
    }

    /// Aborts if the request's live-byte delta since construction exceeds the budget.
    pub fn check(&self) -> PartialVMResult<()> {
        let delta = (self.read_live)() - self.start_live;
        if delta > self.budget as i64 {
            return Err(PartialVMError::new(StatusCode::ABORTED)
                .with_message("Query exceeds size limit".to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    thread_local! {
        static FAKE_LIVE: Cell<i64> = const { Cell::new(0) };
    }
    fn fake_read() -> i64 {
        FAKE_LIVE.with(|c| c.get())
    }
    fn set_live(v: i64) {
        FAKE_LIVE.with(|c| c.set(v));
    }

    #[test]
    fn check_passes_under_budget_and_aborts_over() {
        set_live(1_000);
        let meter = Meter::new(500, fake_read);
        set_live(1_400); // +400 ≤ 500
        assert!(meter.check().is_ok());
        set_live(1_600); // +600 > 500
        assert!(meter.check().is_err());
    }

    #[test]
    fn freed_transients_refund_the_budget() {
        set_live(0);
        let meter = Meter::new(500, fake_read);
        set_live(600); // momentarily over
        assert!(meter.check().is_err());
        set_live(300); // transient freed → back under
        assert!(meter.check().is_ok());
    }
}
