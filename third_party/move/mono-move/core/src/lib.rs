// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod align;
mod error;
mod function;
mod instruction;
pub mod interner;
pub mod native;
mod object_descriptor;
mod prepared_module;
pub mod storage;
pub mod types;

pub use align::{
    align_max, align_up, align_up_u32, checked_align_max, checked_align_up, MAX_ALIGN,
};
pub use error::{ExecutionError, ExecutionErrorKind, ExecutionResult, IntoExecutionError};
pub use function::{
    Code, FrameLayoutInfo, Function, FunctionPtr, SafePointEntry, SortedSafePointEntries,
};
pub use instruction::{
    CallClosureOp, ClosureFuncRef, CodeOffset, DescriptorId, FrameOffset, IntBinaryOp, IntCastOp,
    IntNegateOp, IntOperand, IntShiftOp, IntTy, MicroOp, MicroOpGasSchedule, PackClosureOp,
    ShiftOperand, SizedSlot, CAPTURED_DATA_TAG_MATERIALIZED, CAPTURED_DATA_TAG_OFFSET,
    CAPTURED_DATA_VALUES_OFFSET, CLOSURE_CAPTURED_DATA_PTR_OFFSET, CLOSURE_DATA_SIZE,
    CLOSURE_FUNC_REF_OFFSET, CLOSURE_FUNC_REF_SIZE, CLOSURE_MASK_OFFSET, ENUM_DATA_OFFSET,
    ENUM_TAG_OFFSET, FRAME_METADATA_SIZE, FUNC_REF_PAYLOAD_OFFSET, FUNC_REF_TAG_OFFSET,
    FUNC_REF_TAG_RESOLVED, OBJECT_HEADER_SIZE,
};
pub use interner::{Interner, ModuleId};
pub use object_descriptor::{
    DescriptorProvider, ObjectDescriptor, ObjectDescriptorInner, ObjectDescriptorTable,
    CLOSURE_DESCRIPTOR_ID, RESERVED_DESCRIPTOR_COUNT, TRIVIAL_DESCRIPTOR_ID,
};
pub use prepared_module::{
    FieldTypes, FunctionInstantiationSignature, FunctionSignature, PreparedModule,
};
pub use storage::{
    ModuleProvider, NoResourceProvider, ResourceProvider, ResourceProviderError, StorageRead,
    NO_RESOURCE_PROVIDER,
};
pub use types::{convert_mut_to_immut_ref, strip_ref};
