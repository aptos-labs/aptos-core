// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! New VM runtime PoC with unified stack, bump-allocated heap, and copying GC.

pub(crate) mod heap;
mod interpreter;
pub(crate) mod memory;
mod types;
mod verifier;

pub use interpreter::InterpreterContext;
pub use memory::{
    read_ptr, read_u32, read_u64, vec_elem_ptr, write_ptr, write_u32, write_u64, MemoryRegion,
};
pub use types::{ObjectDescriptor, StepResult, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET};
pub use verifier::{verify_function, VerificationError};
