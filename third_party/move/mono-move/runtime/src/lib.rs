// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoVM runtime implementation.

pub mod error;
mod execution_context;
pub(crate) mod global_storage;
pub(crate) mod heap;
mod interpreter;
mod local_runtime_context;
mod materializing_provider;
pub(crate) mod memory;
mod native_context;
mod transaction_context;
mod types;
mod value_utils;
mod verifier;

pub use error::{RuntimeError, RuntimeStatus};
pub use execution_context::{ExecutionContext, LocalExecutionContext};
pub use interpreter::{InterpreterContext, ResourceWrite};
pub use local_runtime_context::LocalRuntimeContext;
pub use materializing_provider::MaterializingResourceProvider;
pub use memory::{
    read_ptr, read_u32, read_u64, vec_elem_ptr, write_object_header, write_ptr, write_u32,
    write_u64, MemoryRegion,
};
pub use mono_move_core::{
    DescriptorProvider, LayoutProvider, ObjectDescriptor, ObjectDescriptorTable,
    CLOSURE_DESCRIPTOR_ID, TRIVIAL_DESCRIPTOR_ID,
};
pub use native_context::{
    ProductionContextFamily, ProductionNativeContext, ProductionNativeFunction,
    ProductionNativeRegistry,
};
pub use transaction_context::TransactionContext;
pub use types::{StepResult, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET};
pub use verifier::{verify_function, verify_program, VerificationError};
