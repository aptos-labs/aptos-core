// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lowers stackless execution IR to monomorphized micro-ops.

pub mod context;
pub mod display;
mod translate;

pub use context::{build_func_id_map, try_build_context, LoweringContext, SlotInfo};
pub use display::MicroOpsFunctionDisplay;
use mono_move_core::MicroOp;
use move_binary_format::file_format::IdentifierIndex;
pub use translate::lower_function;

/// Result of lowering a single non-generic function.
// TODO: unify with `mono_move_core::Function` once the specializer has access to arenas.
pub struct LoweredFunction {
    /// Function name, as an index into the module's identifier pool.
    pub name_idx: IdentifierIndex,
    /// Gas-instrumented micro-ops.
    pub code: Vec<MicroOp>,
    /// Size of the argument region at the start of the frame.
    pub args_size: usize,
    /// Size of the arguments + locals region.
    pub args_and_locals_size: usize,
    /// Total frame footprint (args + locals + metadata + callee slots).
    pub extended_frame_size: usize,
}

/// Result of lowering an entire module.
pub struct LoweredModule {
    /// Per-definition-index results. `None` for functions that were
    /// not lowered (e.g., generic functions).
    pub functions: Vec<Option<LoweredFunction>>,
}
