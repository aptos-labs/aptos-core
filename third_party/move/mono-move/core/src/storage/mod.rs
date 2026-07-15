// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Storage abstractions for the VM runtime to provide access to data or code.

pub mod module_provider;
pub mod resource_provider;

pub use module_provider::{ModuleProvider, NoModuleProvider};
pub use resource_provider::{
    NoResourceProvider, ResourceProvider, ResourceProviderError, StorageRead,
};
