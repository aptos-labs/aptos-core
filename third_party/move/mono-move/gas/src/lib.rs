// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! ISA-agnostic gas metering abstractions for MonoMove.
//!
//! This crate has **no dependency on any instruction set**. It defines the
//! interfaces and the generic instrumentation pass; concrete instruction sets
//! (micro-ops, stackless IR, …) plug in by implementing four traits:
//!
//! | Trait                     | Purpose                                        |
//! |---------------------------|------------------------------------------------|
//! | [`HasCfgInfo`]            | Identifies basic-block boundaries              |
//! | [`RemapTargets`]          | Rewrites branch targets after charge insertion |
//! | [`GasSchedule<I>`]        | Maps each instruction to its [`InstrCost`]     |
//! | [`GasMeteredInstruction`] | Constructs charge instructions within the ISA  |
//!
//! ## Integration
//!
//! A new instruction set plugs in by adding a `Charge` variant to its
//! instruction type and implementing the four traits above.
//! [`GasInstrumentor::run`] then instruments any `Vec<I>` at compile time.
//!
//! The interpreter handles the charge variant:
//!
//! ```text
//! match instr {
//!     MyOp::Charge { cost } => gas_meter.charge(cost)?,
//!     ...
//! }
//! ```

pub mod cfg;
pub mod instrument;

pub use cfg::{compute_basic_blocks, BasicBlock, HasCfgInfo};
pub use instrument::{GasInstrumentor, GasMeteredInstruction, RemapTargets};

// ---------------------------------------------------------------------------
// Gas meter
// ---------------------------------------------------------------------------

/// Gas exhaustion: the transaction ran out of budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

// ---------------------------------------------------------------------------
// Gas schedule
// ---------------------------------------------------------------------------

/// The cost of a single instruction, as reported by [`GasSchedule::cost`].
#[derive(Debug)]
pub struct InstrCost<I> {
    /// Accumulated into the enclosing `Charge` op for the basic block.
    pub base: u64,

    /// A fully-formed gas charge instruction to insert immediately after
    /// the instruction, if any.
    pub dynamic: Option<I>,
}

impl<I> InstrCost<I> {
    pub fn constant(base: u64) -> Self {
        Self {
            base,
            dynamic: None,
        }
    }
}

/// Maps instructions to their gas cost.
///
/// The instrumentation pass calls [`GasSchedule::cost`] once per instruction
/// and bakes all parameters into the emitted charge ops — the interpreter
/// never consults the schedule at runtime.
///
/// # Constraint
///
/// Branch instructions (those for which [`HasCfgInfo::branch_target`] returns
/// `Some`) must not have a dynamic cost component. The dynamic charge op is
/// inserted immediately after the instruction, so on the taken path execution
/// jumps away and the charge is never reached. For unconditional jumps it is
/// completely unreachable.
pub trait GasSchedule<I> {
    fn cost(&self, instr: &I) -> InstrCost<I>;
}
