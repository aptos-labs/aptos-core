// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Loader for the mono-move module cache.
//!
//! Provides policy-driven entry points that load modules from storage into
//! the [`mono_move_global_context::GlobalContext`]'s executable cache:
//!
//! - [`Loader::load_lazy`] loads just the requested module.
//! - [`Loader::load_package`] loads the entire package containing the
//!   requested module.
//!
//! Policy is determined by on-chain config and is stable within an epoch
//! (policy changes trigger full cache flushes via reconfiguration); see the
//! crate `README.md` for the design rationale. A third policy,
//! `LazyWithTransitiveStructs`, is specified in the README and will be
//! added in a follow-up.

mod hooks;
mod loader;
mod read_set;

pub use hooks::LoaderHooks;
pub use loader::{Loader, LoadingPolicy};
pub use read_set::ExecutableReadSet;
