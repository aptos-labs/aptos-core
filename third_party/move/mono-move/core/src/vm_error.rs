// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Internal error enums for the loader and interpreter subsystems.
//!
//! TODO: consider moving into a separate crate.

use crate::{
    ExecutionErrorKind, GasExhaustedError, IntTy, IntoExecutionError, ResourceProviderError,
};
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

    // TODO: also report how many bytes were free after GC.
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
            | AbortMessageTooLong { .. } => ExecutionErrorKind::RuntimeLimitExceeded,

            BCSEof | BCSInvalidUleb | BCSSequenceTooLong { .. } | BCSRemainingInput { .. } => {
                ExecutionErrorKind::InvalidOperation
            },

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

    /// TODO: temporary until natives are supported.
    #[error("Function IR missing")]
    FunctionIrMissing,

    /// TODO: temporary until nominal types are supported.
    #[error("Failed to lower function: {reason}")]
    LoweringSkipped { reason: &'static str },

    /// TODO: replace once the deserializer has its own error type.
    #[error(transparent)]
    Deserialization(anyhow::Error),

    /// TODO: replace once the verifier has its own error type.
    #[error(transparent)]
    Verification(anyhow::Error),

    /// Catch-all for `ModuleProvider` failures.
    /// TODO: figure out the right error type(s) here.
    #[error(transparent)]
    ModuleProvider(anyhow::Error),

    /// TODO: replace once the global context has its own error type.
    #[error(transparent)]
    GlobalContext(anyhow::Error),

    /// TODO: replace once the specializer has its own error type.
    #[error(transparent)]
    Specializer(anyhow::Error),

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

            // TODO: delegate to the inner errors once they have their own types.
            Deserialization(_)
            | Verification(_)
            | ModuleProvider(_)
            | GlobalContext(_)
            | Specializer(_)
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
