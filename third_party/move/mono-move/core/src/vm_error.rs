// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Internal error enums for the loader and interpreter subsystems.
//!
//! TODO(cleanup): consider moving into a separate crate.

use crate::{
    native::NativeABIError, ExecutionErrorKind, GasExhaustedError, IntTy, IntoExecutionError,
    PreparedModuleError, ResourceProviderError, TypeSubstitutionError,
};
use move_binary_format::errors::VMError;
use move_core_types::account_address::AccountAddress;
use std::fmt;
use thiserror::Error;

pub type RuntimeResult<T> = Result<T, RuntimeError>;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error(transparent)]
    GasExhausted(#[from] GasExhaustedError),

    #[error(transparent)]
    Loader(#[from] LoaderError),

    #[error("{op}.{ty}: overflow")]
    ArithmeticOverflow { op: ArithOp, ty: IntTy },

    #[error("{op}.{ty}: underflow")]
    ArithmeticUnderflow { op: ArithOp, ty: IntTy },

    #[error("{op}.{ty}: division by zero")]
    DivisionByZero { op: ArithOp, ty: IntTy },

    #[error("{op}.{ty}: shift amount {shift_amount} >= bit width {bit_width}")]
    ShiftAmountOutOfRange {
        op: ArithOp,
        ty: IntTy,
        shift_amount: u8,
        bit_width: u32,
    },

    #[error("{op}: under/overflow")]
    ArithmeticUnderOverflow { op: ArithOp },

    #[error("{op}: by zero or overflow")]
    DivisionByZeroOrOverflow { op: ArithOp },

    #[error("Negate.{ty}: Negate of MIN overflows")]
    NegateMinOverflow { ty: IntTy },

    #[error("Cast.{from}->{to}: value out of range for {to}")]
    CastOutOfRange { from: IntTy, to: IntTy },

    #[error("VecPopBack on empty vector")]
    PopFromEmptyVector,

    #[error("VecUnpack: expected {expected} elements, vector has {actual}")]
    VecUnpackLengthMismatch { expected: u64, actual: u64 },

    #[error("{op} index out of bounds: idx={idx} len={len}")]
    VectorIndexOutOfBounds { op: VecOp, idx: u64, len: u64 },

    #[error("{op}: resource does not exist at {addr}")]
    ResourceDoesNotExist {
        op: GlobalStorageOp,
        addr: AccountAddress,
    },

    #[error("MoveTo: resource already exists at {addr}")]
    ResourceAlreadyExists { addr: AccountAddress },

    #[error("enum variant mismatch: runtime variant tag {tag} is not the expected variant (STRUCT_VARIANT_MISMATCH)")]
    EnumVariantMismatch { tag: u64 },

    #[error("stack overflow")]
    StackOverflow,

    // TODO(cleanup): also report how many bytes were free after GC.
    #[error("out of heap memory after GC (requested {requested} bytes)")]
    OutOfHeapMemory { requested: usize },

    #[error("heap_alloc: size {requested} exceeds maximum single allocation size")]
    AllocationTooLarge { requested: usize },

    #[error("alloc_vec: size overflow")]
    VecAllocSizeOverflow,

    #[error("AbortMsg: message is not valid UTF-8")]
    InvalidAbortMessage,

    #[error("AbortMsg: message size {len} exceeds maximum {max}")]
    AbortMessageTooLong { len: usize, max: usize },

    #[error("resource type is too deeply nested to encode as a state key")]
    StateKeyTypeTooDeep,

    #[error("invariant violation: {0}")]
    InvariantViolation(#[from] RuntimeInvariantViolation),

    #[error("resource provider: {0}")]
    ResourceProvider(#[from] ResourceProviderError),

    #[error("BCS deserialize: unexpected end of input")]
    BCSEof,

    #[error("BCS deserialize: malformed ULEB128 length")]
    BCSInvalidUleb,

    #[error("BCS deserialize: sequence length {len} exceeds maximum")]
    BCSSequenceTooLong { len: u64 },

    #[error("BCS deserialize: {remaining} trailing byte(s) after value")]
    BCSRemainingInput { remaining: usize },

    #[error("BCS deserialize: non-canonical bool byte {byte}")]
    BCSInvalidBool { byte: u8 },
}

impl IntoExecutionError for RuntimeError {
    fn kind(&self) -> ExecutionErrorKind {
        use RuntimeError::*;
        match self {
            GasExhausted(_) => ExecutionErrorKind::OutOfGas,

            Loader(e) => e.kind(),

            ArithmeticOverflow { .. }
            | ArithmeticUnderflow { .. }
            | DivisionByZero { .. }
            | ShiftAmountOutOfRange { .. }
            | ArithmeticUnderOverflow { .. }
            | DivisionByZeroOrOverflow { .. }
            | NegateMinOverflow { .. }
            | CastOutOfRange { .. }
            | PopFromEmptyVector
            | VecUnpackLengthMismatch { .. }
            | VectorIndexOutOfBounds { .. }
            | InvalidAbortMessage
            | ResourceDoesNotExist { .. }
            | ResourceAlreadyExists { .. }
            | EnumVariantMismatch { .. } => ExecutionErrorKind::InvalidOperation,

            StackOverflow
            | OutOfHeapMemory { .. }
            | AllocationTooLarge { .. }
            | VecAllocSizeOverflow
            | AbortMessageTooLong { .. }
            | StateKeyTypeTooDeep => ExecutionErrorKind::RuntimeLimitExceeded,

            BCSEof
            | BCSInvalidUleb
            | BCSSequenceTooLong { .. }
            | BCSRemainingInput { .. }
            | BCSInvalidBool { .. } => ExecutionErrorKind::InvalidOperation,

            InvariantViolation(_) => ExecutionErrorKind::InvariantViolation,
            ResourceProvider(e) => e.kind(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Signedness {
    Signed,
    Unsigned,
}

impl fmt::Display for Signedness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Signedness::Signed => write!(f, "signed"),
            Signedness::Unsigned => write!(f, "unsigned"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Shl,
    Shr,
    Negate,
    BitAnd,
    BitOr,
    BitXor,
}

impl fmt::Display for ArithOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArithOp::Add => write!(f, "Add"),
            ArithOp::Sub => write!(f, "Sub"),
            ArithOp::Mul => write!(f, "Mul"),
            ArithOp::Div => write!(f, "Div"),
            ArithOp::Mod => write!(f, "Mod"),
            ArithOp::Shl => write!(f, "Shl"),
            ArithOp::Shr => write!(f, "Shr"),
            ArithOp::Negate => write!(f, "Negate"),
            ArithOp::BitAnd => write!(f, "BitAnd"),
            ArithOp::BitOr => write!(f, "BitOr"),
            ArithOp::BitXor => write!(f, "BitXor"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VecOp {
    LoadElem,
    StoreElem,
    Borrow,
    Swap,
}

impl fmt::Display for VecOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VecOp::LoadElem => write!(f, "VecLoadElem"),
            VecOp::StoreElem => write!(f, "VecStoreElem"),
            VecOp::Borrow => write!(f, "VecBorrow"),
            VecOp::Swap => write!(f, "VecSwap"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalStorageOp {
    BorrowGlobal,
    BorrowGlobalMut,
    MoveFrom,
}

impl fmt::Display for GlobalStorageOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GlobalStorageOp::BorrowGlobal => write!(f, "BorrowGlobal"),
            GlobalStorageOp::BorrowGlobalMut => write!(f, "BorrowGlobalMut"),
            GlobalStorageOp::MoveFrom => write!(f, "MoveFrom"),
        }
    }
}

/// Conditions that should never occur if the compiler, verifier, and
/// runtime maintain their invariants. Surfaced rather than panicked so
/// callers can produce a clean per-transaction outcome and alert
/// operationally on `ExecutionErrorKind::InvariantViolation`.
#[derive(Debug, Error)]
pub enum RuntimeInvariantViolation {
    #[error("pc out of bounds: pc={pc} but function {func_name} has {code_len} instructions")]
    PcOutOfBounds {
        pc: usize,
        func_name: String,
        code_len: usize,
    },

    /// An int op was dispatched to a type the op is not defined for
    /// (e.g. bitwise on signed, shift on signed, negate on unsigned).
    #[error("Int{op} on a {signedness} value is invalid")]
    OperationNotSupportedForType { op: ArithOp, signedness: Signedness },

    #[error("alloc_obj called with non-allocatable descriptor {descriptor_id}")]
    NonAllocatableDescriptor { descriptor_id: u32 },

    #[error("descriptor {descriptor_id} not found in descriptor table")]
    DescriptorNotFound { descriptor_id: u32 },

    #[error("type has no published layout")]
    ValueLayoutNotFound,

    #[error("unreachable: {0}")]
    Unreachable(String),

    /// Invariant violation raised by a native function.
    #[error("native function invariant violation: {0}")]
    Native(String),

    #[error("GC scan: invalid object size {size} (expected non-zero, MAX_ALIGN-byte aligned)")]
    GcInvalidObjectSize { size: usize },

    #[error("GC found forwarding marker in to-space")]
    GcForwardingMarkerInToSpace,

    #[error("CallClosure: null closure pointer")]
    NullClosure,

    #[error("CallClosure: closure_src object has descriptor {descriptor_id}, not the closure descriptor")]
    ClosureSrcNotClosure { descriptor_id: u32 },

    #[error("CallClosure: callee has {num_params} params, exceeds 64-bit mask capacity")]
    TooManyClosureParams { num_params: usize },

    #[error("CallClosure: mask {mask:#b} references parameters beyond callee's {num_params}")]
    ClosureMaskExceedsParams { mask: u64, num_params: usize },

    #[error("CallClosure: packed captured values_size {packed} != resolved callee's captured layout {expected}")]
    ClosureCapturedLayoutMismatch { expected: u32, packed: u32 },

    #[error("CallClosure: null function pointer in closure")]
    NullFuncRefInClosure,

    #[error("CallClosure: unknown func_ref tag {tag}")]
    InvalidClosureFuncRefTag { tag: u8 },

    #[error("CallClosure: null captured_data for closure with captured params")]
    NullCapturedData,

    #[error("CallClosure: provided_args[{provided_idx}].size {provided_size} != callee param_slots[{param_idx}].size {param_size}")]
    ClosureArgSizeMismatch {
        provided_idx: usize,
        provided_size: u32,
        param_idx: usize,
        param_size: u32,
    },

    #[error("CallClosure: not enough provided args")]
    NotEnoughProvidedArgs,

    #[error(
        "CallClosure: {provided} provided_args but only {consumed} non-captured params consumed"
    )]
    ClosureArgsCountMismatch { provided: usize, consumed: usize },

    #[error("resource provider: {0}")]
    ResourceProviderInvariant(String),

    #[error("rollback({requested}): only {available} checkpoint(s) on the stack")]
    RollbackUnderflow { requested: usize, available: usize },

    #[error("enum tag {tag} out of range for {variant_count} variants")]
    EnumTagOutOfRange { tag: u64, variant_count: usize },

    #[error("MoveTo: null source pointer")]
    MoveToNullSource,

    #[error("CallNative: native_idx {idx} out of bounds in registry of size {registry_size}")]
    NativeIdxOutOfBounds { idx: u32, registry_size: usize },

    #[error("a native extension was borrowed when the GC tried to scan its roots")]
    ExtensionBorrowedDuringGC,
}

pub type LoaderResult<T> = Result<T, LoaderError>;

#[derive(Debug, Error)]
pub enum LoaderError {
    #[error(transparent)]
    GasExhausted(#[from] GasExhaustedError),

    #[error("Module {address}::{name} not found")]
    ModuleNotFound {
        address: AccountAddress,
        name: String,
    },

    #[error("Function {address}::{module}::{name} not found")]
    FunctionNotFound {
        address: AccountAddress,
        module: String,
        name: String,
    },

    /// TODO(completeness): temporary until natives are supported.
    #[error("Function IR missing")]
    FunctionIrMissing,

    /// TODO(completeness): temporary until nominal types are supported.
    #[error("Failed to lower function: {reason}")]
    LoweringSkipped { reason: &'static str },

    /// TODO(cleanup): replace once the deserializer has its own error type.
    #[error(transparent)]
    Deserialization(anyhow::Error),

    /// TODO(cleanup): replace once the verifier has its own error type.
    #[error(transparent)]
    Verification(anyhow::Error),

    /// Catch-all for `ModuleProvider` failures.
    /// TODO(cleanup): figure out the right error type(s) here.
    #[error(transparent)]
    ModuleProvider(anyhow::Error),

    /// TODO(cleanup): replace once the global context has its own error type.
    #[error(transparent)]
    GlobalContext(anyhow::Error),

    #[error(transparent)]
    Specializer(#[from] SpecializerError),

    #[error(transparent)]
    InvariantViolation(#[from] LoaderInvariantViolation),
}

impl IntoExecutionError for LoaderError {
    fn kind(&self) -> ExecutionErrorKind {
        use LoaderError::*;
        match self {
            GasExhausted(_) => ExecutionErrorKind::OutOfGas,

            ModuleNotFound { .. } | FunctionNotFound { .. } | FunctionIrMissing => {
                ExecutionErrorKind::LinkingError
            },

            Specializer(e) => e.kind(),

            // TODO(cleanup): delegate to the inner errors once they have their own types.
            Deserialization(_)
            | Verification(_)
            | ModuleProvider(_)
            | GlobalContext(_)
            | LoweringSkipped { .. } => ExecutionErrorKind::Placeholder,

            InvariantViolation(_) => ExecutionErrorKind::InvariantViolation,
        }
    }
}

/// Read-set state-machine and cache-consistency assertions raised by the
/// loader. Surfaced rather than panicked so callers can produce a clean
/// per-transaction outcome and alert operationally on
/// [`ExecutionErrorKind::InvariantViolation`].
#[derive(Debug, Error)]
pub enum LoaderInvariantViolation {
    // ---- read_set transitions ----
    #[error("There should be no entry when marked as pending")]
    EntryAlreadyExists,

    #[error("Module must be recorded as pending")]
    ModuleExpectedPending,

    #[error("Module is already loaded")]
    ModuleAlreadyLoaded,

    #[error("Module must be loaded")]
    ModuleExpectedLoaded,

    #[error("Module must be at least loaded")]
    ModuleExpectedAtLeastLoaded,

    #[error("Module is already metered")]
    ModuleAlreadyMetered,

    #[error("Module must be metered")]
    ModuleExpectedMetered,

    #[error("Module is already ready for lowering")]
    ModuleAlreadyReady,

    // ---- loader cross-checks against the read-set ----
    #[error("All modules in the read-set must be metered")]
    ReadSetEntryNotMetered,

    #[error("All modules in the read-set must be loaded")]
    ReadSetEntryNotLoaded,

    #[error("Target module is not loaded")]
    TargetModuleNotLoaded,

    #[error("Target module is not metered and ready")]
    TargetModuleNotReady,

    #[error("All modules must be present in the read-set")]
    UnexpectedReadSetMiss,

    // ---- function slot ----
    #[error("Function slot has just been set")]
    FunctionSlotEmptyAfterSet,

    // ---- mandatory dependencies ----
    #[error("Mandatory dependencies must be set")]
    MandatoryDepsNotSet,

    #[error("Mandatory dependencies must always be lazy")]
    MandatoryDepsNotLazy,
}

pub type SpecializerResult<T> = Result<T, SpecializerError>;

/// Typed internal error for the specializer, dispatched per pipeline pass.
/// Its input is already verified, so almost every failure is an invariant
/// violation rather than a user-facing error; each pass owns its error enum
/// and the `IntoExecutionError` impl that assigns the public category.
#[derive(Debug, Error)]
pub enum SpecializerError {
    #[error("bytecode verification failed: {0}")]
    Verification(VMError),

    #[error(transparent)]
    ModulePreparation(PreparedModuleError),

    #[error("0x0::test_utils::force_gc must take no arguments and return nothing")]
    ForceGcBadSignature,

    #[error(transparent)]
    SsaConversion(#[from] SsaConversionError),

    #[error(transparent)]
    SlotAlloc(#[from] SlotAllocError),

    #[error(transparent)]
    XferVerifier(#[from] XferVerifierError),

    #[error(transparent)]
    GasInstrumentation(#[from] GasInstrumentationError),

    #[error(transparent)]
    Lowering(#[from] LoweringError),
}

impl IntoExecutionError for SpecializerError {
    fn kind(&self) -> ExecutionErrorKind {
        use SpecializerError::*;
        match self {
            // TODO(cleanup): map to `VerificationFailed`/`DeserializationFailed`
            // once those categories exist.
            Verification(_) | ModulePreparation(_) => ExecutionErrorKind::Placeholder,
            ForceGcBadSignature => ExecutionErrorKind::InvariantViolation,
            SsaConversion(e) => e.kind(),
            SlotAlloc(e) => e.kind(),
            XferVerifier(e) => e.kind(),
            GasInstrumentation(e) => e.kind(),
            Lowering(e) => e.kind(),
        }
    }
}

impl From<LoweringError> for LoaderError {
    fn from(err: LoweringError) -> Self {
        LoaderError::Specializer(SpecializerError::from(err))
    }
}

pub type SsaConversionResult<T> = Result<T, SsaConversionError>;

#[derive(Debug, Error)]
pub enum SsaConversionError {
    // TODO(security): consider verifying this at publish time?
    #[error("too many SSA values (Vid u16 overflow)")]
    TooManySsaValues,

    #[error(transparent)]
    TypeSubstitution(#[from] TypeSubstitutionError),

    #[error("verified bytecode must end with a terminator")]
    MissingTerminator,

    #[error("operand stack underflow")]
    StackUnderflow,

    #[error("Vid id {vid} out of range")]
    VidOutOfRange { vid: u16 },

    #[error("expected a Vid slot on the operand stack")]
    ExpectedVidOnStack,

    #[error("operand stack must be empty at a block boundary")]
    StackNotEmptyAtBlockBoundary,

    #[error("expected a struct type")]
    ExpectedStructType,

    #[error("expected an enum type")]
    ExpectedEnumType,

    #[error("CallClosure signature is empty")]
    ClosureSignatureEmpty,

    #[error("CallClosure signature must start with a Function type")]
    ClosureSignatureNotFunction,

    #[error("expected a reference type")]
    ExpectedReferenceType,

    #[error("expected a mutable reference type")]
    ExpectedMutableReference,
}

impl IntoExecutionError for SsaConversionError {
    fn kind(&self) -> ExecutionErrorKind {
        use SsaConversionError::*;
        match self {
            TooManySsaValues => ExecutionErrorKind::RuntimeLimitExceeded,
            TypeSubstitution(e) => e.kind(),
            MissingTerminator
            | StackUnderflow
            | VidOutOfRange { .. }
            | ExpectedVidOnStack
            | StackNotEmptyAtBlockBoundary
            | ExpectedStructType
            | ExpectedEnumType
            | ClosureSignatureEmpty
            | ClosureSignatureNotFunction
            | ExpectedReferenceType
            | ExpectedMutableReference => ExecutionErrorKind::InvariantViolation,
        }
    }
}

pub type SlotAllocResult<T> = Result<T, SlotAllocError>;

#[derive(Debug, Error)]
pub enum SlotAllocError {
    #[error("VID type not found during SSA allocation")]
    VidTypeNotFound,

    #[error("vid_type called on a non-Vid slot")]
    VidTypeOnNonVidSlot,
}

impl IntoExecutionError for SlotAllocError {
    fn kind(&self) -> ExecutionErrorKind {
        use SlotAllocError::*;
        match self {
            VidTypeNotFound | VidTypeOnNonVidSlot => ExecutionErrorKind::InvariantViolation,
        }
    }
}

pub type XferVerifierResult<T> = Result<T, XferVerifierError>;

#[derive(Debug, Error)]
pub enum XferVerifierError {
    #[error("post-optimize Xfer verifier: block {block}, instr {instr}: {inner}")]
    XferCallStructural {
        block: usize,
        instr: usize,
        inner: Box<XferVerifierError>,
    },

    #[error(
        "arg positionality: args[{arg_idx}] resolves to Xfer({got}), expected Xfer({arg_idx})"
    )]
    XferArgPositionality { arg_idx: usize, got: u16 },

    #[error("return Xfer prefix: rets[{ret_idx}] resolves to Xfer({got}) after a non-Xfer ret")]
    XferReturnPrefix { ret_idx: usize, got: u16 },

    #[error("return monotonicity: rets[{ret_idx}] = Xfer({got}) <= prev Xfer({prev})")]
    XferReturnNotMonotonic { ret_idx: usize, got: u16, prev: u16 },

    #[error("post-optimize Xfer verifier: block {block}, instr {instr}: use of Xfer({xfer}) with no live def earlier in this block")]
    XferUseWithoutLiveDef {
        block: usize,
        instr: usize,
        xfer: u16,
    },

    #[error("post-optimize Xfer verifier: block {block}, instr {instr}: Xfer({xfer}) bound at call boundary but not consumed as args[{xfer}]")]
    XferBoundNotConsumed {
        block: usize,
        instr: usize,
        xfer: u16,
    },

    #[error("post-optimize Xfer verifier: block {block}: Xfer({xfer}) bound at block end (Xfer lifetimes must be block-local)")]
    XferBoundAtBlockEnd { block: usize, xfer: u16 },
}

impl IntoExecutionError for XferVerifierError {
    fn kind(&self) -> ExecutionErrorKind {
        use XferVerifierError::*;
        match self {
            XferCallStructural { .. }
            | XferArgPositionality { .. }
            | XferReturnPrefix { .. }
            | XferReturnNotMonotonic { .. }
            | XferUseWithoutLiveDef { .. }
            | XferBoundNotConsumed { .. }
            | XferBoundAtBlockEnd { .. } => ExecutionErrorKind::InvariantViolation,
        }
    }
}

pub type GasInstrumentationResult<T> = Result<T, GasInstrumentationError>;

#[derive(Debug, Error)]
pub enum GasInstrumentationError {
    #[error(transparent)]
    TypeSubstitution(#[from] TypeSubstitutionError),

    #[error("expected a reference type")]
    ExpectedReferenceType,

    #[error("Xfer({xfer}) read without a prior call-return binding")]
    XferReadWithoutBinding { xfer: u16 },

    #[error("Vid slot in post-allocation IR")]
    VidInPostAllocationIr,

    #[error("field owner is not a struct type")]
    FieldOwnerNotStruct,

    #[error("variant owner is not an enum type")]
    VariantOwnerNotEnum,

    #[error("enum definition not found")]
    EnumDefinitionNotFound,

    #[error("type is not an enum")]
    NotAnEnum,

    #[error("call return {ret_idx} has no matching signature type")]
    CallReturnNoSignatureType { ret_idx: usize },

    #[error("CallClosure signature is empty")]
    ClosureSignatureEmpty,

    #[error("CallClosure signature must start with a Function type")]
    ClosureSignatureNotFunction,
}

impl IntoExecutionError for GasInstrumentationError {
    fn kind(&self) -> ExecutionErrorKind {
        use GasInstrumentationError::*;
        match self {
            TypeSubstitution(e) => e.kind(),
            ExpectedReferenceType
            | XferReadWithoutBinding { .. }
            | VidInPostAllocationIr
            | FieldOwnerNotStruct
            | VariantOwnerNotEnum
            | EnumDefinitionNotFound
            | NotAnEnum
            | CallReturnNoSignatureType { .. }
            | ClosureSignatureEmpty
            | ClosureSignatureNotFunction => ExecutionErrorKind::InvariantViolation,
        }
    }
}

pub type LoweringResult<T> = Result<T, LoweringError>;

#[derive(Debug, Error)]
pub enum LoweringError {
    /// Boxed to break the `LoaderError` ⇄ `LoweringError` type cycle.
    #[error(transparent)]
    Loader(Box<LoaderError>),

    #[error(transparent)]
    TypeSubstitution(#[from] TypeSubstitutionError),

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

impl From<LoaderError> for LoweringError {
    fn from(err: LoaderError) -> Self {
        LoweringError::Loader(Box::new(err))
    }
}

impl IntoExecutionError for LoweringError {
    fn kind(&self) -> ExecutionErrorKind {
        use LoweringError::*;
        match self {
            Loader(e) => e.kind(),
            TypeSubstitution(e) => e.kind(),
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
