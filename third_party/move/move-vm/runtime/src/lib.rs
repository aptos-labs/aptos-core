// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
#![deny(deprecated)]

//! The core Move VM logic.

pub mod data_cache;
mod interpreter;
mod loader;
pub mod logging;
pub mod move_vm;
pub mod native_extensions;
pub mod native_functions;
#[macro_use]
pub mod tracing;
pub mod config;
pub mod module_traversal;

// Only include debugging functionality in debug builds
#[cfg(any(debug_assertions, feature = "debugging"))]
mod debug;

mod access_control;
mod frame;
mod frame_type_cache;
mod reentrancy_checker;
mod runtime_type_checks;
mod storage;

pub use loader::{Function, LoadedFunction, LoadedFunctionOwner, Module, Script};
pub use storage::{
    code_storage::{ambassador_impl_CodeStorage, CodeStorage},
    dependencies_gas_charging::{
        check_dependencies_and_charge_gas, check_script_dependencies_and_check_gas,
        check_type_tag_dependencies_and_charge_gas,
    },
    environment::{
        ambassador_impl_WithRuntimeEnvironment, RuntimeEnvironment, WithRuntimeEnvironment,
    },
    implementations::{
        unsync_code_storage::{AsUnsyncCodeStorage, UnsyncCodeStorage},
        unsync_module_storage::{AsUnsyncModuleStorage, BorrowedOrOwned, UnsyncModuleStorage},
    },
    module_storage::{ambassador_impl_ModuleStorage, AsFunctionValueExtension, ModuleStorage},
    publishing::{StagingModuleStorage, VerifiedModuleBundle},
};
