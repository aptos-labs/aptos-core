// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Gas metering primitives for MonoMove.

use crate::error::{ExecutionErrorKind, IntoExecutionError};
use thiserror::Error;

/// Why a charge failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum MeteringError {
    /// The transaction ran out of budget.
    #[error("out of gas")]
    OutOfGas,
    /// The interrupt source asked the transaction to stop, e.g. because
    /// Block-STM aborted the running incarnation. The output must be
    /// discarded.
    #[error("execution interrupted")]
    Interrupted,
}

impl IntoExecutionError for MeteringError {
    fn kind(&self) -> ExecutionErrorKind {
        match self {
            MeteringError::OutOfGas => ExecutionErrorKind::OutOfGas,
            MeteringError::Interrupted => ExecutionErrorKind::Interrupted,
        }
    }
}

/// Tells a running transaction whether to stop, e.g. because Block-STM aborted
/// its incarnation. Once it returns `true`, it keeps returning `true`.
pub trait InterruptSource {
    fn interrupt_requested(&self) -> bool;
}

/// An interrupt source that never asks to stop.
pub struct NoInterrupts;

impl InterruptSource for NoInterrupts {
    fn interrupt_requested(&self) -> bool {
        false
    }
}

/// Gas units between two interrupt polls.
//
// TODO(metering): derive the interval from the gas schedule.
pub const INTERRUPT_INTERVAL_GAS_UNITS: u64 = 10_000;

/// Polls an interrupt source once every `interval_gas_units` gas units charged.
#[derive(Clone, Copy)]
pub struct InterruptListener<'a> {
    source: &'a dyn InterruptSource,
    interval_gas_units: u64,
}

impl<'a> InterruptListener<'a> {
    pub fn new(source: &'a dyn InterruptSource, interval_gas_units: u64) -> Self {
        Self {
            source,
            interval_gas_units,
        }
    }
}

/// A flat-budget gas meter, called by the interpreter, the loader, and natives
/// at charge points.
///
/// Charges draw from the current interval, and a reserve refills it. Without
/// an interrupt listener, the whole budget is one interval. With one, an
/// interval holds at most `interval_gas_units`, and the meter polls the
/// listener's source each time an interval runs out. Either way, a charge
/// fails with out-of-gas at the same point as with a single counter.
pub struct GasMeter<'a> {
    /// Gas left in the current interval.
    interval_balance: u64,
    /// Gas left beyond the current interval.
    reserve: u64,
    interrupt_listener: Option<InterruptListener<'a>>,
}

impl<'a> GasMeter<'a> {
    pub fn new(budget: u64) -> Self {
        Self::from_balance(budget, None)
    }

    /// Meter with `u64::MAX` budget, for tests and benches that don't
    /// exercise gas exhaustion.
    pub fn with_max_budget() -> Self {
        Self::new(u64::MAX)
    }

    /// Polls `listener` for interrupts once per interval.
    pub fn with_interrupt_listener(self, listener: InterruptListener<'a>) -> Self {
        Self::from_balance(self.balance(), Some(listener))
    }

    /// Meter with `u64::MAX` budget and this meter's interrupt listener, for
    /// code that runs unmetered but must still stop when asked.
    pub fn unbounded(&self) -> Self {
        Self::from_balance(u64::MAX, self.interrupt_listener)
    }

    fn from_balance(balance: u64, interrupt_listener: Option<InterruptListener<'a>>) -> Self {
        let interval_balance = match interrupt_listener {
            Some(listener) => balance.min(listener.interval_gas_units),
            None => balance,
        };
        Self {
            interval_balance,
            reserve: balance - interval_balance,
            interrupt_listener,
        }
    }

    /// Deducts `amount` units from the current interval. Returns `false`,
    /// leaving the meter unchanged, if the interval cannot cover it.
    #[inline(always)]
    fn try_charge_in_interval(&mut self, amount: u64) -> bool {
        match self.interval_balance.checked_sub(amount) {
            Some(interval_balance) => {
                self.interval_balance = interval_balance;
                true
            },
            None => false,
        }
    }

    /// Deducts `amount` units. If the current interval cannot cover it, polls
    /// for an interrupt and then starts the next interval. On failure the
    /// meter is unchanged.
    #[inline(always)]
    pub fn charge(&mut self, amount: u64) -> Result<(), MeteringError> {
        if self.try_charge_in_interval(amount) {
            return Ok(());
        }
        self.charge_across_intervals(amount)
    }

    #[cold]
    #[inline(never)]
    fn charge_across_intervals(&mut self, amount: u64) -> Result<(), MeteringError> {
        if let Some(listener) = self.interrupt_listener {
            if listener.source.interrupt_requested() {
                return Err(MeteringError::Interrupted);
            }
        }
        let left = self
            .balance()
            .checked_sub(amount)
            .ok_or(MeteringError::OutOfGas)?;
        *self = Self::from_balance(left, self.interrupt_listener);
        Ok(())
    }

    /// Remaining gas balance.
    pub fn balance(&self) -> u64 {
        // Never overflows: the sum is the balance, which started as a `u64`.
        self.interval_balance + self.reserve
    }

    /// Reset the remaining budget, keeping the interrupt listener.
    pub fn reset(&mut self, budget: u64) {
        *self = Self::from_balance(budget, self.interrupt_listener);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::cell::Cell;

    /// Counts polls, and asks to stop from its `fire_at`-th poll on.
    struct CountingSource {
        fire_at: u64,
        polls: Cell<u64>,
    }

    impl CountingSource {
        fn new(fire_at: u64) -> Self {
            Self {
                fire_at,
                polls: Cell::new(0),
            }
        }

        fn never() -> Self {
            Self::new(u64::MAX)
        }
    }

    impl InterruptSource for CountingSource {
        fn interrupt_requested(&self) -> bool {
            let polls = self.polls.get() + 1;
            self.polls.set(polls);
            polls >= self.fire_at
        }
    }

    fn budget() -> impl Strategy<Value = u64> {
        prop_oneof![
            Just(0),
            Just(1),
            Just(INTERRUPT_INTERVAL_GAS_UNITS - 1),
            Just(INTERRUPT_INTERVAL_GAS_UNITS),
            Just(INTERRUPT_INTERVAL_GAS_UNITS + 1),
            Just(u64::MAX),
            0..1_000u64,
            any::<u64>(),
        ]
    }

    fn interval_gas_units() -> impl Strategy<Value = Option<u64>> {
        prop::option::of(prop_oneof![
            Just(1),
            Just(INTERRUPT_INTERVAL_GAS_UNITS),
            1..64u64,
            any::<u64>()
        ])
    }

    fn amount() -> impl Strategy<Value = u64> {
        prop_oneof![
            Just(0),
            Just(1),
            Just(INTERRUPT_INTERVAL_GAS_UNITS - 1),
            Just(INTERRUPT_INTERVAL_GAS_UNITS),
            Just(INTERRUPT_INTERVAL_GAS_UNITS + 1),
            Just(2 * INTERRUPT_INTERVAL_GAS_UNITS),
            Just(u64::MAX),
            0..100u64,
            any::<u64>(),
        ]
    }

    proptest! {
        #[test]
        fn meter_matches_flat_counter(
            budget in budget(),
            interval_gas_units in interval_gas_units(),
            amounts in prop::collection::vec(amount(), 0..64),
        ) {
            let source = CountingSource::never();
            let mut meter = GasMeter::new(budget);
            if let Some(interval_gas_units) = interval_gas_units {
                meter = meter.with_interrupt_listener(InterruptListener::new(
                    &source,
                    interval_gas_units,
                ));
            }
            let mut flat = budget;
            // Units charged since the last poll, not counting the charge that
            // polled.
            let mut since_poll = 0u64;
            for amount in amounts {
                let polls = source.polls.get();
                let result = meter.charge(amount);
                if source.polls.get() == polls {
                    if result.is_ok() {
                        since_poll += amount;
                    }
                } else {
                    prop_assert_eq!(source.polls.get(), polls + 1);
                    since_poll = 0;
                }

                let expected = flat.checked_sub(amount).ok_or(MeteringError::OutOfGas);
                prop_assert_eq!(result, expected.map(|_| ()));
                if let Ok(left) = expected {
                    flat = left;
                }
                prop_assert_eq!(meter.balance(), flat);

                match interval_gas_units {
                    Some(interval_gas_units) => {
                        prop_assert!(meter.interval_balance <= interval_gas_units);
                        prop_assert!(since_poll <= interval_gas_units);
                    },
                    None => {
                        // Without a listener, the whole budget is one interval.
                        prop_assert_eq!(meter.reserve, 0);
                        prop_assert_eq!(source.polls.get(), 0);
                    },
                }
            }
        }
    }

    #[test]
    fn single_charge_spans_several_intervals() {
        let source = CountingSource::never();
        let mut meter =
            GasMeter::new(100).with_interrupt_listener(InterruptListener::new(&source, 10));
        assert_eq!(meter.charge(4), Ok(()));
        assert_eq!(source.polls.get(), 0);

        // Takes the 6 units left in the interval and 29 from the reserve,
        // polling once.
        assert_eq!(meter.charge(35), Ok(()));
        assert_eq!(source.polls.get(), 1);
        assert_eq!(meter.balance(), 61);
        assert_eq!(meter.interval_balance, 10);
        assert_eq!(meter.reserve, 51);
    }

    #[test]
    fn interrupt_leaves_meter_unchanged() {
        let source = CountingSource::new(2);
        let mut meter =
            GasMeter::new(100).with_interrupt_listener(InterruptListener::new(&source, 10));
        assert_eq!(meter.charge(15), Ok(()));
        assert_eq!(meter.charge(15), Err(MeteringError::Interrupted));
        assert_eq!(source.polls.get(), 2);
        assert_eq!(meter.balance(), 85);
        assert_eq!(meter.interval_balance, 10);
    }

    #[test]
    fn interrupt_wins_over_out_of_gas() {
        let source = CountingSource::new(1);
        let mut meter =
            GasMeter::new(5).with_interrupt_listener(InterruptListener::new(&source, 10));
        assert_eq!(meter.charge(6), Err(MeteringError::Interrupted));
        assert_eq!(meter.balance(), 5);
    }

    #[test]
    fn reset_and_unbounded_keep_listener() {
        let source = CountingSource::never();
        let mut meter =
            GasMeter::new(5).with_interrupt_listener(InterruptListener::new(&source, 10));
        meter.reset(25);
        assert_eq!(meter.balance(), 25);
        assert_eq!(meter.interval_balance, 10);

        let mut unbounded = meter.unbounded();
        assert_eq!(unbounded.balance(), u64::MAX);
        assert_eq!(unbounded.interval_balance, 10);
        assert_eq!(unbounded.charge(11), Ok(()));
        assert_eq!(source.polls.get(), 1);
    }
}
