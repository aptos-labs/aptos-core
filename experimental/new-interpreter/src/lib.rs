// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! New VM interpreter PoC with unified stack, bump-allocated heap, and copying GC.

pub mod heap;
pub mod instruction;
pub mod interpreter;
pub mod memory;
pub mod types;
pub mod verifier;

pub use instruction::Instruction;
pub use memory::{
    read_ptr, read_u32, read_u64, vec_elem_ptr, write_ptr, write_u32, write_u64, MemoryRegion,
};
pub use types::*;
