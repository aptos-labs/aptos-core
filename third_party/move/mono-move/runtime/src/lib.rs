// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoVM runtime implementation.

pub mod error;
mod execution_context;
pub(crate) mod global_storage;
pub(crate) mod heap;
mod interpreter;
mod local_runtime_context;
pub(crate) mod memory;
pub mod serialize;
mod transaction_context;
mod types;
mod verifier;

pub use error::{RuntimeError, RuntimeStatus, ValueSerializationError};
pub use execution_context::{ExecutionContext, LocalExecutionContext};
pub use heap::pinned_roots::{PinGuard, PinnedRoots};
pub use interpreter::InterpreterContext;
pub use local_runtime_context::LocalRuntimeContext;
pub use memory::{
    read_ptr, read_u32, read_u64, vec_elem_ptr, write_object_header, write_ptr, write_u32,
    write_u64, MemoryRegion,
};
pub use mono_move_core::{
    DescriptorProvider, ObjectDescriptor, ObjectDescriptorTable, CLOSURE_DESCRIPTOR_ID,
    TRIVIAL_DESCRIPTOR_ID,
};
pub use serialize::{serialize_value, serialized_value_size};
pub use transaction_context::TransactionContext;
pub use types::{StepResult, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET};
pub use verifier::{verify_function, verify_program, VerificationError};
