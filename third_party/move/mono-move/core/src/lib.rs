// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub mod align;
mod error;
mod function;
mod gas;
mod instruction;
pub mod interner;
pub mod memory;
pub mod native;
mod object_descriptor;
mod prepared_module;
pub mod root_pool;
pub mod storage;
pub mod types;
pub mod value_layout;
pub mod vm_error;

pub use align::{
    align_max, align_up, align_up_u32, checked_align_max, checked_align_up, checked_align_up_u32,
    MAX_ALIGN,
};
pub use error::{ExecutionError, ExecutionErrorKind, ExecutionResult, IntoExecutionError};
pub use function::{
    Code, FrameLayoutInfo, Function, FunctionPtr, SafePointEntry, SortedSafePointEntries,
};
pub use gas::{GasExhaustedError, GasMeter};
pub use instruction::{
    captured_values_size, next_captured_value_offset, CallClosureOp, ClosureFuncRef, CmpKind,
    CodeOffset, DescriptorId, FrameOffset, IntBinaryOp, IntCastOp, IntCmpOp, IntNegateOp,
    IntOperand, IntShiftOp, IntTy, JumpIntCmpOp, JumpValueCmpOp, JumpValueRefCmpOp, MicroOp,
    PackClosureOp, ShiftOperand, SizedSlot, ValueCmpOp, ValueRefCmpOp, VecPackOp, VecUnpackOp,
    CAPTURED_DATA_TAG_MATERIALIZED, CAPTURED_DATA_TAG_OFFSET, CAPTURED_DATA_VALUES_OFFSET,
    CAPTURED_DATA_VALUES_SIZE_OFFSET, CLOSURE_CAPTURED_DATA_PTR_OFFSET, CLOSURE_DATA_SIZE,
    CLOSURE_FUNC_REF_OFFSET, CLOSURE_FUNC_REF_SIZE, CLOSURE_MASK_OFFSET, ENUM_DATA_OFFSET,
    ENUM_TAG_OFFSET, FRAME_METADATA_SIZE, FUNC_REF_PAYLOAD_OFFSET, FUNC_REF_TAG_OFFSET,
    FUNC_REF_TAG_RESOLVED, FUNC_REF_TAG_UNRESOLVED, OBJECT_HEADER_SIZE, VEC_DATA_OFFSET,
    VEC_LENGTH_OFFSET,
};
pub use interner::{
    struct_tag_of, type_tag_of, view_function_ref, view_module_id, FunctionRef,
    InternedFunctionRef, Interner, ModuleId, TypeSubstitutionError,
};
pub use move_binary_format::file_format::ConstantPoolIndex;
pub use object_descriptor::{
    DescriptorProvider, ObjectDescriptor, ObjectDescriptorInner, ObjectDescriptorTable,
    CLOSURE_DESCRIPTOR_ID, RESERVED_DESCRIPTOR_COUNT, TRIVIAL_DESCRIPTOR_ID,
};
pub use prepared_module::{
    intern_sig_token, FieldTypes, FunctionInstantiationSignature, FunctionSignature,
    PreparedModule, PreparedModuleError,
};
pub use root_pool::{ObjectHandle, ReferenceHandle, RootPool};
pub use storage::{
    ModuleProvider, NoModuleProvider, NoResourceProvider, ResourceProvider, ResourceProviderError,
    StorageRead,
};
pub use types::{convert_mut_to_immut_ref, strip_ref};
pub use value_layout::{
    reserved_layout_id, reserved_layouts, FieldValueLayout, LayoutFlags, LayoutId, LayoutKind,
    LayoutProvider, ValueLayout, ValueLayoutTable,
};
pub use vm_error::{
    ArithOp, GasInstrumentationError, GasInstrumentationResult, GlobalStorageOp, LoaderError,
    LoaderInvariantViolation, LoaderResult, LoweringError, LoweringResult, RuntimeError,
    RuntimeInvariantViolation, RuntimeResult, Signedness, SlotAllocError, SlotAllocResult,
    SpecializerError, SpecializerResult, SsaConversionError, SsaConversionResult, VecOp,
    XferVerifierError, XferVerifierResult,
};
