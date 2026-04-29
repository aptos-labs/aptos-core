// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod executable;
mod function;
mod instruction;
pub mod interner;
mod transaction_context;
pub mod types;

pub use executable::{EnumType, Executable, ExecutableId, StructType, VariantFields};
pub use function::{FrameLayoutInfo, Function, SafePointEntry, SortedSafePointEntries};
pub use instruction::{
    CallClosureOp, ClosureFuncRef, CodeOffset, DescriptorId, FrameOffset, MicroOp,
    MicroOpGasSchedule, PackClosureOp, SizedSlot, CAPTURED_DATA_TAG_MATERIALIZED,
    CAPTURED_DATA_TAG_OFFSET, CAPTURED_DATA_VALUES_OFFSET, CLOSURE_CAPTURED_DATA_PTR_OFFSET,
    CLOSURE_FUNC_REF_OFFSET, CLOSURE_FUNC_REF_SIZE, CLOSURE_MASK_OFFSET, CLOSURE_OBJECT_SIZE,
    ENUM_DATA_OFFSET, ENUM_TAG_OFFSET, FRAME_METADATA_SIZE, FUNC_REF_PAYLOAD_OFFSET,
    FUNC_REF_TAG_OFFSET, FUNC_REF_TAG_RESOLVED, OBJECT_HEADER_SIZE, STRUCT_DATA_OFFSET,
};
pub use interner::{walk_sig_token, Interner, StructResolver};
pub use transaction_context::{FunctionResolver, NoopTransactionContext, TransactionContext};
pub use types::{convert_mut_to_immut_ref, strip_ref};
