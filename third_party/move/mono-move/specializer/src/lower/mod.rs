// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lowers stackless execution IR to monomorphized micro-ops.

pub mod context;
pub mod display;
pub mod gc_layout;
mod parallel_copy;
mod translate;

pub use context::{try_build_context, BuildContextOutcome, LoweringContext, SlotInfo};
pub use display::MicroOpsFunctionDisplay;
pub use translate::lower_function;
