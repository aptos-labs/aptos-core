// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod common;
mod local;
mod remote;

use aptos_api_types::MoveModule;
use enum_dispatch::enum_dispatch;
pub use local::LocalTypeAccessorBuilder;
pub use remote::RemoteTypeAccessorBuilder;

/// Defines functions common to all `TypeAccessorBuilder`s.
pub trait TypeAccessorBuilderTrait {
    /// Add modules that we have already retrieved.
    fn add_modules(self, modules: Vec<MoveModule>) -> Self;

    /// Add a module that we have already retreived.
    fn add_module(self, module: MoveModule) -> Self;
}

/// This enum has as its variants all possible implementations of ModuleRetrieverTrait.
#[enum_dispatch(TypeAccessorBuilderTrait)]
#[derive(Clone, Debug)]
pub enum TypeAccessor {
    LocalTypeAccessorBuilder,
    RemoteTypeAccessorBuilder,
}
