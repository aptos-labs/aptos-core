// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native function interface for MonoMove.
pub mod abi;
pub mod context;
pub mod production;
pub mod registry;
pub mod result;
pub mod value;

pub use abi::{FrameSlot, NativeABI, NativeABIError};
pub use context::NativeContext;
pub use production::{
    ProductionContextFamily, ProductionNativeContext, ProductionNativeFunction,
    ProductionNativeRegistry,
};
pub use registry::{
    NativeContextFamily, NativeFunction, NativeIdx, NativeName, NativeRegistry,
    NativeRegistryError, NativeResolver, NoNatives,
};
pub use result::{NativeStatus, VMInternalError};
pub use value::VMValue;
