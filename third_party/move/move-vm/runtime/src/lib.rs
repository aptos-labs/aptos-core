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
mod frame_type_cache;
mod runtime_type_checks;
mod storage;

pub use loader::{Function, LoadedFunction, Module, Script};
#[cfg(any(test, feature = "testing"))]
pub use storage::implementations::unreachable_code_storage;
pub use storage::{
    code_storage::{ambassador_impl_CodeStorage, CodeStorage},
    environment::{
        ambassador_impl_WithRuntimeEnvironment, RuntimeEnvironment, WithRuntimeEnvironment,
    },
    implementations::{
        unsync_code_storage::{AsUnsyncCodeStorage, UnsyncCodeStorage},
        unsync_module_storage::{AsUnsyncModuleStorage, BorrowedOrOwned, UnsyncModuleStorage},
    },
    module_storage::{ambassador_impl_ModuleStorage, AsFunctionValueExtension, ModuleStorage},
    publishing::{StagingModuleStorage, VerifiedModuleBundle},
    ty_layout_converter::{LayoutConverter, StorageLayoutConverter},
};
