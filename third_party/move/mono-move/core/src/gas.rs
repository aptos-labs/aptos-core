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

/// A simple flat-budget gas meter, called by the interpreter at charge points.
pub struct GasMeter {
    remaining: u64,
}

impl GasMeter {
    pub fn new(budget: u64) -> Self {
        Self { remaining: budget }
    }

    /// Meter with `u64::MAX` budget, for tests and benches that don't
    /// exercise gas exhaustion.
    pub fn with_max_budget() -> Self {
        Self::new(u64::MAX)
    }

    /// Deduct `amount` units, returning `Err(GasExhaustedError)` if exhausted.
    pub fn charge(&mut self, amount: u64) -> Result<(), GasExhaustedError> {
        self.remaining = self
            .remaining
            .checked_sub(amount)
            .ok_or(GasExhaustedError)?;
        Ok(())
    }

    /// Remaining gas balance.
    pub fn balance(&self) -> u64 {
        self.remaining
    }

    /// Reset the remaining budget.
    pub fn reset(&mut self, budget: u64) {
        self.remaining = budget;
    }
}
