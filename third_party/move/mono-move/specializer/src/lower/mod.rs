// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lowers stackless execution IR to monomorphized micro-ops.

pub mod context;
pub mod display;
mod translate;

pub use context::{build_func_id_map, try_build_context, LoweringContext, SlotInfo};
pub use display::MicroOpsFunctionDisplay;
pub use translate::lower_function;
