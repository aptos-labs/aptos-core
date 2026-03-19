// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! ISA-agnostic gas metering abstractions for MonoMove.
//!
//! This crate has **no dependency on any instruction set**. It defines the
//! interfaces and the generic instrumentation pass; concrete instruction sets
//! (micro-ops, stackless IR, …) plug in by implementing four traits:
//!
//! | Trait              | Purpose                                        |
//! |--------------------|------------------------------------------------|
//! | [`HasCfgInfo`]     | Identifies basic-block boundaries              |
//! | [`RemapTargets`]   | Rewrites branch targets after charge insertion |
//! | [`GasSchedule<I>`] | Maps each instruction to its [`InstrCost`]     |
//! | [`GasInstr`]       | Constructs charge instructions within the ISA  |
//!
//! ## Integration
//!
//! A new instruction set plugs in by adding `ChargeBlock` and
//! `ChargeVariable` variants to its instruction type and implementing
//! the four traits above. [`GasInstrumentation::run`] then instruments
//! any `Vec<I>` at compile time.
//!
//! The interpreter handles the two charge variants:
//!
//! ```text
//! match instr {
//!     MyOp::ChargeBlock { cost } => gas_meter.charge(cost)?,
//!     MyOp::ChargeVariable { per_unit, slot } =>
//!         gas_meter.charge(per_unit.saturating_mul(frame.read_u64(slot)))?,
//!     ...
//! }
//! ```

pub mod cfg;
pub mod instrument;

pub use cfg::{compute_basic_blocks, BasicBlock, HasCfgInfo};
pub use instrument::{GasInstr, GasInstrumentation, RemapTargets};

// ---------------------------------------------------------------------------
// Gas meter
// ---------------------------------------------------------------------------

/// Gas exhaustion: the transaction ran out of budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GasExhausted;

/// Gas metering interface called by the interpreter at charge points.
pub trait GasMeter {
    /// Deduct `amount` units, returning `Err(GasExhausted)` if exhausted.
    fn charge(&mut self, amount: u64) -> Result<(), GasExhausted>;
    /// Remaining gas balance.
    fn balance(&self) -> u64;
}

/// A simple flat-budget gas meter.
pub struct BudgetMeter {
    remaining: u64,
}

impl BudgetMeter {
    pub fn new(budget: u64) -> Self {
        Self { remaining: budget }
    }
}

impl GasMeter for BudgetMeter {
    fn charge(&mut self, amount: u64) -> Result<(), GasExhausted> {
        self.remaining = self.remaining.checked_sub(amount).ok_or(GasExhausted)?;
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
///
/// - `Static(c)`: the entire cost `c` is accumulated into the enclosing
///   `ChargeBlock`.
/// - `Dynamic { base, per_unit, slot }`: `base` is accumulated into
///   `ChargeBlock`; the instrumentation pass also inserts a `ChargeVariable`
///   op immediately after the instruction that charges
///   `per_unit * frame[slot]` at runtime.
#[derive(Debug)]
pub enum InstrCost<S> {
    Static(u64),
    Dynamic { base: u64, per_unit: u64, slot: S },
}

/// Maps instructions to their gas cost.
///
/// The instrumentation pass calls [`GasSchedule::cost`] once per instruction
/// and bakes all parameters into the emitted charge ops — the interpreter
/// never consults the schedule at runtime.
pub trait GasSchedule<I> {
    /// The slot type used to address runtime values for dynamic charges.
    type Slot;
    fn cost(&self, instr: &I) -> InstrCost<Self::Slot>;
}
