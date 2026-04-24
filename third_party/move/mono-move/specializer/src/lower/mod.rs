// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lowers stackless execution IR to monomorphized micro-ops.

pub mod context;
pub mod display;
mod translate;

pub use context::{try_build_context, LoweringContext, SlotInfo};
pub use display::MicroOpsFunctionDisplay;
use mono_move_core::MicroOp;
use move_binary_format::file_format::{FunctionHandleIndex, IdentifierIndex};
pub use translate::lower_function;

/// Result of lowering a single non-generic function.
// TODO: unify with `mono_move_core::Function` once the specializer has access to arenas.
pub struct LoweredFunction {
    /// Function name, as an index into the module's identifier pool.
    pub name_idx: IdentifierIndex,
    /// Handle index of this function in the defining module.
    pub handle_idx: FunctionHandleIndex,
    /// Gas-instrumented micro-ops.
    pub code: Vec<MicroOp>,
    /// Byte size of each parameter, in declaration order. `param_sizes_sum`
    /// is the sum of these.
    pub param_sizes: Vec<u32>,
    /// Size of the argument region at the start of the frame.
    pub param_sizes_sum: usize,
    /// Size of the arguments + locals region.
    pub param_and_local_sizes_sum: usize,
    /// Total frame footprint (args + locals + metadata + callee slots).
    pub extended_frame_size: usize,
}

/// Result of lowering an entire module. Only successfully lowered
/// functions appear; functions that were skipped (e.g., generic or
/// native) are omitted. Each entry carries its own `handle_idx`, so the
/// vector order is not meaningful.
pub struct LoweredModule {
    pub functions: Vec<LoweredFunction>,
}
