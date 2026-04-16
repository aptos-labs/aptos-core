// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! ISA-agnostic gas metering abstractions for MonoMove.
//!
//! This crate has **no dependency on any instruction set**. It defines the
//! interfaces and the generic instrumentation pass; concrete instruction sets
//! (micro-ops, stackless IR, …) plug in by implementing three traits:
//!
//! | Trait                 | Purpose                                                        |
//! |-----------------------|----------------------------------------------------------------|
//! | [`HasCfgInfo`]        | Identifies basic-block boundaries                             |
//! | [`GasSchedule<I>`]    | Maps each instruction to its static base cost                 |
//! | [`ChargeOnJump`]      | Writes destination-block costs into jump instructions         |
//!
//! ## Integration
//!
//! A new instruction set plugs in by implementing the three traits above.
//! [`GasInstrumentor::run`] then instruments any `Vec<I>` at compile time,
//! returning the annotated instruction sequence and the entry-block cost:
//!
//! ```text
//! let (ops, entry_gas) = GasInstrumentor::new(MySchedule).run(ops);
//! // store entry_gas in the function's metadata
//! ```
//!
//! At runtime the interpreter charges gas when executing a jump (before
//! entering the destination block) and at function entry (using `entry_gas`):
//!
//! ```text
//! match instr {
//!     MyOp::Jump { target, gas } => { gas_meter.charge(gas)?; pc = target; }
//!     MyOp::CondJump { target, gas_taken, gas_fallthrough } => {
//!         if cond { gas_meter.charge(gas_taken)?; pc = target; }
//!         else    { gas_meter.charge(gas_fallthrough)?; pc += 1; }
//!     }
//!     ...
//! }
//! // at function call time:
//! gas_meter.charge(callee.entry_gas)?;
//! ```

pub mod cfg;
pub mod instrument;

pub use cfg::{compute_basic_blocks, BasicBlock, HasCfgInfo};
pub use instrument::{ChargeOnJump, GasInstrumentor};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Gas meter
// ---------------------------------------------------------------------------

/// Gas exhaustion: the transaction ran out of budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("out of gas")]
pub struct GasExhaustedError;

/// Gas metering interface called by the interpreter at charge points.
pub trait GasMeter {
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

// ---------------------------------------------------------------------------
// Gas schedule
// ---------------------------------------------------------------------------

/// Maps instructions to their static base gas cost.
///
/// The instrumentation pass calls [`GasSchedule::cost`] once per instruction
/// and accumulates the results into a block-level cost that is baked into each
/// block's entry jumps — the interpreter never consults the schedule at
/// runtime.
pub trait GasSchedule<I> {
    fn cost(&self, instr: &I) -> u64;
}
