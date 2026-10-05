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
mod value_cmp;
mod value_conv;

pub use error::{ArithOp, GlobalStorageOp, ReportedIntValue, RuntimeError, RuntimeStatus, VecOp};
pub use global_storage::{ResourceReadWriteSet, WriteClass};
pub use heap::{FrozenHeap, Heap, SharedArena};
pub use interpreter::{
    CallBuilder, CallError, CompletedCall, InterpreterContext, InterpreterOptions, SessionEffects,
};
pub use memory::{
    read_ptr, read_u32, read_u64, vec_elem_ptr, write_object_header, write_ptr, write_u32,
    write_u64, MemoryRegion,
};
pub use mono_move_core::{
    assert_well_formed, check_well_formedness, ConstantPoolProvider, ObjectDescriptor,
    ObjectDescriptorTable, WellFormednessError, WellFormednessProvider,
};
pub use native_context::{
    ProductionContextFamily, ProductionNativeContext, ProductionNativeFunction,
    ProductionNativeRegistry,
};
pub use types::{DEFAULT_HEAP_SIZE, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET};
pub use value_conv::bcs::{deserialize_into, serialize};
