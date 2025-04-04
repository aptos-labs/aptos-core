// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

mod access_specifier_loader;

mod function;
pub use function::{Function, LoadedFunction, LoadedFunctionOwner};
pub(crate) use function::{
    FunctionHandle, FunctionInstantiation, LazyLoadedFunction, LazyLoadedFunctionState,
};

mod modules;
pub use modules::Module;
pub(crate) use modules::{StructVariantInfo, VariantFieldInfo};

mod script;
pub use script::Script;

mod type_loader;
use type_loader::intern_type;
