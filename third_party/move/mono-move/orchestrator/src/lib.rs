// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Orchestrator: drives module loading by resolving types, running the
//! specializer, and assembling executables.

mod builder;

use anyhow::Result;
pub use builder::ExecutableBuilder;
use mono_move_global_context::{ExecutionGuard, LoadedModule};
use move_binary_format::CompiledModule;

/// Build a loaded module from a compiled module.
///
/// Orchestrates the full pipeline:
/// 1. Resolve struct/enum types via the execution guard's interner
/// 2. Run the specializer (destack → lower → gas instrument)
/// 3. Assemble the loaded module (IR + executable + dependencies)
pub fn build_executable(
    guard: &ExecutionGuard<'_>,
    module: CompiledModule,
) -> Result<Box<LoadedModule>> {
    ExecutableBuilder::new(guard, module)?.build()
}
