// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Gas metering primitives for MonoMove.

use crate::error::{ExecutionErrorKind, IntoExecutionError};
use thiserror::Error;

/// Gas exhaustion: the transaction ran out of budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("out of gas")]
pub struct GasExhaustedError;

impl IntoExecutionError for GasExhaustedError {
    fn kind(&self) -> ExecutionErrorKind {
        ExecutionErrorKind::OutOfGas
    }
}

/// Execution was interrupted on request of the block executor, e.g. because
/// Block-STM aborted the running incarnation. The output must be discarded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("execution interrupted")]
pub struct InterruptedError;

impl IntoExecutionError for InterruptedError {
    fn kind(&self) -> ExecutionErrorKind {
        ExecutionErrorKind::Interrupted
    }
}

/// Chunk size for meters that poll for interrupts.
//
// TODO(metering): derive the interval from the gas schedule.
pub const POLL_INTERVAL: u64 = 10_000;

/// A flat-budget gas meter, called by the interpreter at charge points.
///
/// Charges draw from the current chunk, and a reserve refills it. Without
/// interrupt polling, the whole budget is one chunk. With polling, a chunk
/// holds at most `chunk_size` units, and the interpreter polls for interrupts
/// each time one runs out. Either way, a charge fails at the same point as
/// with a single counter.
pub struct GasMeter {
    /// Gas left in the current chunk.
    chunk: u64,
    /// Gas left beyond the current chunk.
    reserve: u64,
    /// Maximum size of a chunk, or 0 if the meter does not poll.
    chunk_size: u64,
}

impl GasMeter {
    pub fn new(budget: u64) -> Self {
        Self::split(budget, 0)
    }

    /// Meter with `u64::MAX` budget, for tests and benches that don't
    /// exercise gas exhaustion.
    pub fn with_max_budget() -> Self {
        Self::new(u64::MAX)
    }

    /// Splits the balance into chunks of at most `chunk_size` units, so the
    /// interpreter polls for interrupts once per chunk. A `chunk_size` of 0
    /// turns polling off.
    pub fn with_interrupt_polling(self, chunk_size: u64) -> Self {
        Self::split(self.balance(), chunk_size)
    }

    fn split(balance: u64, chunk_size: u64) -> Self {
        let chunk = if chunk_size == 0 {
            balance
        } else {
            balance.min(chunk_size)
        };
        Self {
            chunk,
            reserve: balance - chunk,
            chunk_size,
        }
    }

    /// Maximum size of a chunk, or 0 if the meter does not poll.
    pub fn chunk_size(&self) -> u64 {
        self.chunk_size
    }

    /// Whether the interpreter should poll for interrupts when a chunk runs
    /// out.
    pub fn polls_interrupts(&self) -> bool {
        self.chunk_size != 0
    }

    /// Deduct `amount` units from the current chunk. Returns `false`, leaving
    /// the meter unchanged, if the chunk cannot cover it.
    #[inline(always)]
    pub fn try_charge_in_chunk(&mut self, amount: u64) -> bool {
        match self.chunk.checked_sub(amount) {
            Some(chunk) => {
                self.chunk = chunk;
                true
            },
            None => false,
        }
    }

    /// Deduct `amount` units, returning `Err(GasExhaustedError)` if exhausted.
    /// Starts the next chunk if the current one cannot cover `amount`.
    #[inline]
    pub fn charge(&mut self, amount: u64) -> Result<(), GasExhaustedError> {
        if self.try_charge_in_chunk(amount) {
            return Ok(());
        }
        self.charge_across_chunks(amount)
    }

    #[cold]
    #[inline(never)]
    fn charge_across_chunks(&mut self, amount: u64) -> Result<(), GasExhaustedError> {
        let left = self
            .balance()
            .checked_sub(amount)
            .ok_or(GasExhaustedError)?;
        *self = Self::split(left, self.chunk_size);
        Ok(())
    }

    /// Remaining gas balance.
    pub fn balance(&self) -> u64 {
        // Never overflows: the sum is the balance, which started as a `u64`.
        self.chunk + self.reserve
    }

    /// Reset the remaining budget, keeping the chunk size.
    pub fn reset(&mut self, budget: u64) {
        *self = Self::split(budget, self.chunk_size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn budget() -> impl Strategy<Value = u64> {
        prop_oneof![
            Just(0),
            Just(1),
            Just(POLL_INTERVAL - 1),
            Just(POLL_INTERVAL),
            Just(POLL_INTERVAL + 1),
            Just(u64::MAX),
            0..1_000u64,
            any::<u64>(),
        ]
    }

    fn chunk_size() -> impl Strategy<Value = u64> {
        prop_oneof![
            Just(0),
            Just(1),
            Just(POLL_INTERVAL),
            1..64u64,
            any::<u64>()
        ]
    }

    fn amount() -> impl Strategy<Value = u64> {
        prop_oneof![
            Just(0),
            Just(1),
            Just(POLL_INTERVAL - 1),
            Just(POLL_INTERVAL),
            Just(POLL_INTERVAL + 1),
            Just(2 * POLL_INTERVAL),
            Just(u64::MAX),
            0..100u64,
            any::<u64>(),
        ]
    }

    proptest! {
        #[test]
        fn chunked_meter_matches_flat_counter(
            budget in budget(),
            chunk_size in chunk_size(),
            amounts in prop::collection::vec(amount(), 0..64),
        ) {
            let mut meter = GasMeter::new(budget).with_interrupt_polling(chunk_size);
            let mut flat = budget;
            // Units charged from the current chunk since it started.
            let mut in_chunk = 0u64;
            for amount in amounts {
                let fast = meter.try_charge_in_chunk(amount);
                let result = if fast {
                    in_chunk += amount;
                    Ok(())
                } else {
                    in_chunk = 0;
                    meter.charge(amount)
                };

                let expected = flat.checked_sub(amount).ok_or(GasExhaustedError);
                prop_assert_eq!(result, expected.map(|_| ()));
                if let Ok(left) = expected {
                    flat = left;
                }
                prop_assert_eq!(meter.balance(), flat);

                if meter.polls_interrupts() {
                    prop_assert!(meter.chunk <= chunk_size);
                    prop_assert!(in_chunk <= chunk_size);
                } else {
                    // A meter that does not poll never leaves its first chunk.
                    prop_assert_eq!(meter.reserve, 0);
                    prop_assert!(fast || result.is_err());
                }
            }
        }
    }

    #[test]
    fn reset_keeps_chunk_size() {
        let mut meter = GasMeter::new(5).with_interrupt_polling(10);
        meter.reset(25);
        assert_eq!(meter.balance(), 25);
        assert_eq!(meter.chunk, 10);
        assert_eq!(meter.chunk_size(), 10);
    }
}
