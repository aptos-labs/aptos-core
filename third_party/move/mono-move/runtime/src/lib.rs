// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoVM runtime implementation.

pub mod error;
pub(crate) mod global_storage;
pub(crate) mod heap;
mod interpreter;
pub(crate) mod memory;
mod native_context;
mod types;
mod value_utils;
mod verifier;

pub use error::{RuntimeError, RuntimeStatus};
pub use heap::Heap;
pub use interpreter::InterpreterContext;
pub use memory::{
    read_ptr, read_u32, read_u64, vec_elem_ptr, write_object_header, write_ptr, write_u32,
    write_u64, MemoryRegion,
};
pub use mono_move_core::{ObjectDescriptor, ObjectDescriptorTable};
pub use native_context::{
    ProductionContextFamily, ProductionNativeContext, ProductionNativeFunction,
    ProductionNativeRegistry,
};
pub use types::{VEC_DATA_OFFSET, VEC_LENGTH_OFFSET};
pub use value_utils::{deserialize_into, serialize};
pub use verifier::{verify_function, verify_program, VerificationError};
