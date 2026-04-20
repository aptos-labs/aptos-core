// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

mod executable;
mod function;
mod instruction;
mod transaction_context;
pub mod types;

pub use executable::{
    EnumType, Executable, ExecutableId, ExecutableSlot, MandatoryDependencies, StructType,
    VariantFields,
};
pub use function::{FrameLayoutInfo, Function, SafePointEntry, SortedSafePointEntries};
pub use instruction::{
    CodeOffset, DescriptorId, FrameOffset, MicroOp, MicroOpGasSchedule, ENUM_DATA_OFFSET,
    ENUM_TAG_OFFSET, FRAME_METADATA_SIZE, OBJECT_HEADER_SIZE, STRUCT_DATA_OFFSET,
};
pub use transaction_context::{FunctionResolver, NoopTransactionContext, TransactionContext};
