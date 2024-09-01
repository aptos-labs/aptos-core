// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
#![deny(deprecated)]

//! The core Move VM logic.
//!
//! It is a design goal for the Move VM to be independent of the Diem blockchain, so that
//! other blockchains can use it as well. The VM isn't there yet, but hopefully will be there
//! soon.

pub mod data_cache;
mod interpreter;
mod loader;
pub mod logging;
pub mod move_vm;
pub mod native_extensions;
pub mod native_functions;
mod runtime;
pub mod session;
#[macro_use]
pub mod tracing;
pub mod config;
pub mod module_traversal;

// Only include debugging functionality in debug builds
#[cfg(any(debug_assertions, feature = "debugging"))]
mod debug;

mod access_control;
mod storage;

pub use loader::{LoadedFunction, Module, Script};
pub use storage::{
    code_storage::{ambassador_impl_CodeStorage, deserialize_script, script_hash, CodeStorage},
    environment::{RuntimeEnvironment, WithRuntimeEnvironment},
    implementations::{
        unreachable_code_storage::UnreachableCodeStorage,
        unsync_code_storage::{IntoUnsyncCodeStorage, UnsyncCodeStorage},
        unsync_module_storage::{
            IntoUnsyncModuleStorage, LocalModuleBytesStorage, UnsyncModuleStorage,
        },
    },
    module_storage::{ambassador_impl_ModuleStorage, ModuleBytesStorage, ModuleStorage},
    publishing::TemporaryModuleStorage,
    use_loader_v1_based_on_env,
};
