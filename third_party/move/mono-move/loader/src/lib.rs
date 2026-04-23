// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Loader for the MonoMove module cache.
//!
//! Provides policy-driven entry points that load modules from storage into
//! long-living cache and local transaction read-set. Supported policies:
//!
//! - [`LoadingPolicy::Lazy`]: loads just the requested module. Functions are
//!   lowered depending on the lowering policy:
//!
//!   1. [`LoweringPolicy::Lazy`]: loads just the requested module. Lowering of
//!      any function that needs external information is deferred to the first
//!      call.
//!
//! - [`LoadingPolicy::Package`] loads every module in the requested module's
//!   package atomically. Lowering is lazy: any function that needs information
//!   outside the package is lowered during the first call.

mod loader;
mod module_provider;
mod read_set;

pub use loader::{Loader, LoadingPolicy, LoweringPolicy};
pub use module_provider::ModuleProvider;
pub use read_set::ExecutableReadSet;
