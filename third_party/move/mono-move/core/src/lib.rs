// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod align;
mod executable;
mod execution_context;
mod function;
mod instruction;
pub mod interner;
mod prepared_module;
pub mod types;

pub use align::{
    align_max, align_up, align_up_u32, checked_align_max, checked_align_up, MAX_ALIGN,
};
pub use executable::{EnumType, Executable, ExecutableId, StructType, VariantFields};
pub use execution_context::{ExecutionContext, FunctionResolver, LocalExecutionContext};
pub use function::{FrameLayoutInfo, Function, SafePointEntry, SortedSafePointEntries};
pub use instruction::{
    CallClosureOp, ClosureFuncRef, CodeOffset, DescriptorId, FrameOffset, MicroOp,
    MicroOpGasSchedule, PackClosureOp, SizedSlot, CAPTURED_DATA_TAG_MATERIALIZED,
    CAPTURED_DATA_TAG_OFFSET, CAPTURED_DATA_VALUES_OFFSET, CLOSURE_CAPTURED_DATA_PTR_OFFSET,
    CLOSURE_DATA_SIZE, CLOSURE_FUNC_REF_OFFSET, CLOSURE_FUNC_REF_SIZE, CLOSURE_MASK_OFFSET,
    ENUM_DATA_OFFSET, ENUM_TAG_OFFSET, FRAME_METADATA_SIZE, FUNC_REF_PAYLOAD_OFFSET,
    FUNC_REF_TAG_OFFSET, FUNC_REF_TAG_RESOLVED, OBJECT_HEADER_SIZE,
};
pub use interner::Interner;
pub use prepared_module::PreparedModule;
pub use types::{convert_mut_to_immut_ref, strip_ref};
