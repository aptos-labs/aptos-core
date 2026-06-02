// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Gas metering primitives for MonoMove.

use thiserror::Error;

// ---------------------------------------------------------------------------
// Gas meter
// ---------------------------------------------------------------------------

/// Gas exhaustion: the transaction ran out of budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("out of gas")]
pub struct GasExhaustedError;

/// Gas metering interface called by the interpreter at charge points.
///
/// The `'static` bound is required by the native function interface.
///
/// TODO: remove this trait in favour of a single concrete gas meter.
pub trait GasMeter: 'static {
    /// Deduct `amount` units, returning `Err(GasExhaustedError)` if exhausted.
    fn charge(&mut self, amount: u64) -> Result<(), GasExhaustedError>;
    /// Remaining gas balance.
    fn balance(&self) -> u64;
}

/// A simple flat-budget gas meter.
pub struct SimpleGasMeter {
    remaining: u64,
}

impl SimpleGasMeter {
    pub fn new(budget: u64) -> Self {
        Self { remaining: budget }
    }

    /// Reset the remaining budget.
    pub fn reset(&mut self, budget: u64) {
        self.remaining = budget;
    }
}

impl GasMeter for SimpleGasMeter {
    fn charge(&mut self, amount: u64) -> Result<(), GasExhaustedError> {
        self.remaining = self
            .remaining
            .checked_sub(amount)
            .ok_or(GasExhaustedError)?;
        Ok(())
    }

    fn balance(&self) -> u64 {
        self.remaining
    }
}

/// A no-op gas meter for testing.
pub struct NoOpGasMeter;

impl GasMeter for NoOpGasMeter {
    fn charge(&mut self, _amount: u64) -> Result<(), GasExhaustedError> {
        Ok(())
    }

    fn balance(&self) -> u64 {
        u64::MAX
    }
}
