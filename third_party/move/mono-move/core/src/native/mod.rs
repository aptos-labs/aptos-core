// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native function interface for MonoMove.
pub mod abi;
pub mod context;
pub mod extension;
pub mod registry;
pub mod result;
pub mod value;

// The root pool lives at the crate root; re-exported here for native authors.
pub use crate::root_pool::{ObjectHandle, ReferenceHandle, RootPool};
pub use abi::{FrameSlot, NativeABI, NativeABIError};
pub use context::NativeContext;
pub use extension::{NativeExtension, NativeExtensions};
pub use registry::{
    NativeContextFamily, NativeFunction, NativeIdx, NativeName, NativeRegistry,
    NativeRegistryError, NativeResolver, NoNatives,
};
pub use result::{native_invariant_violation, NativeStatus};
pub use value::{Boxed, Opaque, Ref, TableHandle, VMValue, Vector};
