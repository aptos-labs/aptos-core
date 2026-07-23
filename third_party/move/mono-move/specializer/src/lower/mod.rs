// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lowers stackless execution IR to monomorphized micro-ops.

use mono_move_core::{native::NativeABIError, ExecutionErrorKind, IntoExecutionError};
use thiserror::Error;

pub mod context;
pub mod gc_layout;
mod lower_utils;
mod parallel_copy;
mod translate;

pub use context::{try_build_context, BuildContextOutcome, LoweringContext, LoweringOutcome};

#[derive(Debug, Error)]
enum LoweringError {
    #[error("native call-site ABI is malformed: {0}")]
    NativeAbi(#[from] NativeABIError),

    // ---- type-shape assertions ----
    #[error("expected a reference type")]
    ExpectedReferenceType,

    #[error("CallClosure signature is empty")]
    ClosureSignatureEmpty,

    #[error("CallClosure signature must start with a Function type")]
    ClosureSignatureNotFunction,

    // ---- post-allocation IR sanity ----
    #[error("Vid slot in post-allocation IR")]
    VidInPostAllocationIr,

    #[error("scratch slot required when emitting 2+ parallel copies")]
    ScratchSlotRequiredForParallelCopy,

    // ---- GC layout derivation ----
    #[error("nominal type has no layout populated")]
    LayoutNotPopulated,

    #[error("type parameter reached gc_layout — try_build_context should have skipped")]
    TypeParamReachedGcLayout,

    #[error("layout id does not resolve to a layout")]
    LayoutIdUnresolved,

    #[error("gc_layout: field offset {field_offset} + inner offset {inner_offset} overflows u32")]
    GcFieldOffsetOverflow {
        field_offset: u32,
        inner_offset: u32,
    },

    // ---- layout / size resolution ----
    #[error("{label} has no concrete size")]
    NoConcreteSize { label: &'static str },

    #[error("{op}: struct type has no populated layout")]
    StructLayoutNotPopulated { op: &'static str },

    #[error("{op}: nominal type is not a struct (no field layouts)")]
    NominalTypeNotStruct { op: &'static str },

    #[error("{op}: field layout id does not resolve")]
    FieldLayoutIdUnresolved { op: &'static str },

    #[error("field index {pos} out of range for struct")]
    FieldIndexOutOfRange { pos: usize },

    // ---- variant field resolution ----
    #[error("{op}: no derived layout for enum")]
    EnumLayoutNotDerived { op: &'static str },

    #[error("variant field index out of range")]
    VariantFieldIndexOutOfRange,

    #[error("variant field handle has no variants")]
    VariantFieldHandleNoVariants,

    // ---- constants ----
    #[error(
        "LdConst at constant pool index {idx}: expected {expected}-byte constant data, got {got}"
    )]
    LdConstBadLength {
        idx: u16,
        expected: usize,
        got: usize,
    },

    #[error("LdConst at constant pool index {idx}: constant type is not permitted by the bytecode verifier")]
    LdConstTypeNotPermitted { idx: u16 },

    // ---- control flow ----
    #[error("conditional terminator in final block has no fallthrough block")]
    FinalBlockNoFallthrough,

    #[error("conditional branch at fixup index {idx} has no fallthrough label")]
    ConditionalBranchNoFallthrough { idx: usize },

    #[error("unexpected non-branch op at fixup index {idx}")]
    UnexpectedNonBranchOpAtFixup { idx: usize },

    #[error("unresolved label L{label}")]
    UnresolvedLabel { label: u16 },

    // ---- arithmetic / casts ----
    #[error("cast source must be an integer type")]
    CastSourceNotInteger,

    #[error("bitwise op on a signed value is invalid")]
    BitwiseOnSignedValue,

    #[error("shift op requires an unsigned non-u64 integer type")]
    ShiftRequiresUnsignedNonU64,

    #[error("unexpected op in arith/bitwise lowering arm")]
    UnexpectedOpInArithArm,

    #[error("unexpected op in shift lowering arm")]
    UnexpectedOpInShiftArm,

    #[error("BinaryOpImm imm must be bool")]
    ImmMustBeBool,

    #[error("Negate requires a signed integer type")]
    NegateRequiresSignedInt,

    #[error("u64 fast path received a wide imm — ill-typed IR")]
    U64FastPathWideImm,

    #[error("shift immediate must be u8")]
    ShiftImmNotU8,

    #[error("expected an integer type")]
    ExpectedIntegerType,

    #[error("bool ImmValue cannot be an integer operand")]
    BoolImmNotInteger,

    // ---- comparisons ----
    #[error("equality is not supported for this operand type")]
    EqualityUnsupportedType,

    #[error("ordering comparison on a non-scalar operand is ill-typed")]
    OrderingOnNonScalar,

    #[error("operand type has no comparison lowering")]
    ComparisonNoLowering,

    // ---- pack / unpack ----
    #[error("{op}: value count {provided} does not match field count {expected}")]
    FieldCountMismatch {
        op: &'static str,
        provided: usize,
        expected: usize,
    },

    #[error("{op}: neither reverse nor forward emit is overlap-safe")]
    NotOverlapSafe { op: &'static str },

    #[error("CallClosure has no closure operand")]
    CallClosureNoOperand,

    // ---- global storage ----
    #[error("{op}: box-pointer slot not reserved")]
    BoxPtrSlotNotReserved { op: &'static str },

    #[error("{op}: no descriptor published for the resource type (its layout may be unresolved)")]
    ResourceTypeNoDescriptor { op: &'static str },

    #[error("{op}: no descriptor published for this vector type (element may be generic or have unresolved layout)")]
    VectorTypeNoDescriptor { op: &'static str },

    // ---- enum variants ----
    #[error("{op}: variant ordinal {ordinal} out of range")]
    VariantOrdinalOutOfRange { op: &'static str, ordinal: usize },

    #[error("{op}: dst/src aliases but no enum-pointer scratch reserved")]
    EnumPtrScratchMissing { op: &'static str },

    #[error("{op}: no scratch slot reserved")]
    VariantFieldScratchMissing { op: &'static str },

    #[error("Xfer({xfer}) read without a prior def in this block")]
    XferReadWithoutDef { xfer: u16 },
}

impl IntoExecutionError for LoweringError {
    fn kind(&self) -> ExecutionErrorKind {
        use LoweringError::*;
        match self {
            NativeAbi(e) => e.kind(),
            ExpectedReferenceType
            | ClosureSignatureEmpty
            | ClosureSignatureNotFunction
            | VidInPostAllocationIr
            | ScratchSlotRequiredForParallelCopy
            | LayoutNotPopulated
            | TypeParamReachedGcLayout
            | LayoutIdUnresolved
            | GcFieldOffsetOverflow { .. }
            | NoConcreteSize { .. }
            | StructLayoutNotPopulated { .. }
            | NominalTypeNotStruct { .. }
            | FieldLayoutIdUnresolved { .. }
            | FieldIndexOutOfRange { .. }
            | EnumLayoutNotDerived { .. }
            | VariantFieldIndexOutOfRange
            | VariantFieldHandleNoVariants
            | LdConstBadLength { .. }
            | LdConstTypeNotPermitted { .. }
            | FinalBlockNoFallthrough
            | ConditionalBranchNoFallthrough { .. }
            | UnexpectedNonBranchOpAtFixup { .. }
            | UnresolvedLabel { .. }
            | CastSourceNotInteger
            | BitwiseOnSignedValue
            | ShiftRequiresUnsignedNonU64
            | UnexpectedOpInArithArm
            | UnexpectedOpInShiftArm
            | ImmMustBeBool
            | NegateRequiresSignedInt
            | U64FastPathWideImm
            | ShiftImmNotU8
            | ExpectedIntegerType
            | BoolImmNotInteger
            | EqualityUnsupportedType
            | OrderingOnNonScalar
            | ComparisonNoLowering
            | FieldCountMismatch { .. }
            | NotOverlapSafe { .. }
            | CallClosureNoOperand
            | BoxPtrSlotNotReserved { .. }
            | ResourceTypeNoDescriptor { .. }
            | VectorTypeNoDescriptor { .. }
            | VariantOrdinalOutOfRange { .. }
            | EnumPtrScratchMissing { .. }
            | VariantFieldScratchMissing { .. }
            | XferReadWithoutDef { .. } => ExecutionErrorKind::InvariantViolation,
        }
    }
}
