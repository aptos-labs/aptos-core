// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]
#![deny(deprecated)]

//! The core Move VM logic.

pub mod data_cache;
mod interpreter;
mod interpreter_caches;
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
mod runtime_ref_checks;
mod runtime_type_checks;
mod storage;

pub use loader::{Function, LoadedFunction, LoadedFunctionOwner, Module, Script};
pub use storage::{
    code_storage::CodeStorage,
    dependencies_gas_charging::check_dependencies_and_charge_gas,
    environment::{
        ambassador_impl_WithRuntimeEnvironment, RuntimeEnvironment, WithRuntimeEnvironment,
    },
    implementations::{
        unsync_code_storage::{AsUnsyncCodeStorage, UnsyncCodeStorage},
        unsync_module_storage::{AsUnsyncModuleStorage, BorrowedOrOwned, UnsyncModuleStorage},
    },
    layout_cache::{LayoutCache, LayoutCacheEntry, LayoutCacheHit},
    loader::{
        eager::EagerLoader,
        lazy::LazyLoader,
        traits::{
            FunctionDefinitionLoader, InstantiatedFunctionLoader, LegacyLoaderConfig, Loader,
            ScriptLoader, StructDefinitionLoader,
        },
    },
    module_storage::{
        ambassador_impl_ModuleStorage, AsFunctionValueExtension, FunctionValueExtensionAdapter,
        ModuleStorage,
    },
    publishing::{StagingModuleStorage, VerifiedModuleBundle},
    ty_layout_converter::LayoutWithDelayedFields,
};

// TODO(lazy-loading): revisit this macro in favour of a callback or an enum.
#[macro_export]
macro_rules! dispatch_loader {
    ($module_storage:expr, $loader:ident, $dispatch:stmt) => {
        if $crate::WithRuntimeEnvironment::runtime_environment($module_storage)
            .vm_config()
            .enable_lazy_loading
        {
            let $loader = $crate::LazyLoader::new($module_storage);
            $dispatch
        } else {
            let $loader = $crate::EagerLoader::new($module_storage);
            $dispatch
        }
    };
}
