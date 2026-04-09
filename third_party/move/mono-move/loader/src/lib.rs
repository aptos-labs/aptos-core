// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-transaction module loader.
//!
//! The loader is the first stage of the module loading pipeline. When a module
//! at the specified ID is loaded, the following happens:
//!   1. Based on the "loading policy", other modules in addition to the
//!      requested module may be pre-fetched.
//!   2. Every module that is not yet in cache, is fetched from storage,
//!      deserialized, verified and converted to execution format. Loader
//!      returns all such modules.
//!   3. For every module load attempt, gas is charged. Gas is charged at all
//!      times whether the module was cached or not.
//!
//! Importantly, for every module we record the set of all other modules that
//! it requires. For example, the policy can be that immediate dependencies
//! are always loaded. IDs of these modules is recorded in the parent module.
//!
//! During loading, accessed modules are recorded in a per-transaction read-set
//! for gas charging and Block-STM conflict detection.

mod hooks;
pub use hooks::LoaderHooks;

mod loader;
pub use loader::{LoadedExecutable, Loader};

pub mod read_set;
