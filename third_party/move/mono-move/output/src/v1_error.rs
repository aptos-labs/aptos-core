// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Describes a MonoMove error the way V1 would have.
//!
//! V1 is the MoveVM in `move-vm-runtime` that MonoMove (V2) replaces.
//!
//! [`describe`] maps a type-erased [`VMInternalError`] to the [`V1ErrorInfo`]
//! V1 would have produced for the same fault: its status code, sub-status, and
//! message. Consumers needing V1-shaped errors share this one mapping instead
//! of keeping their own.
//!
//! Not every message can be reproduced. Most match V1's byte for byte, some
//! faults carry no message at all, and a few keep MonoMove's own text because
//! V1 builds its message from state MonoMove does not have. [`V1Message`]
//! records which of the three a message is, so callers comparing against V1
//! can tell when a mismatch is expected.
//!
//! TODO(cleanup): render transactional-test `VMError`s from this mapping
//! together with the attached error location.
//!
//! TODO(cleanup): **exact parity with V1 is not a goal.** These statuses
//! become the `VMStatus` a transaction commits with, so for now they are what
//! MonoMove reports, and matching V1 is what makes differential testing
//! possible. MonoMove may later report errors its own way: V1 collapses
//! distinct faults into one status and freezes detail into messages, and a new
//! representation can fix both. Diverging on purpose also means changing the
//! replay benchmark, which compares the two VMs by exact `TransactionStatus`
//! equality and would report the divergence as a mismatch.

use mono_move_core::{GasExhaustedError, IntTy, VMInternalError};
use mono_move_loader::LoaderError;
use mono_move_runtime::{ArithOp, GlobalStorageOp, ReportedIntValue, RuntimeError};
use move_core_types::vm_status::StatusCode;
use move_vm_types::values::{INDEX_OUT_OF_BOUNDS, POP_EMPTY_VEC, VEC_UNPACK_PARITY_MISMATCH};

/// How a [`V1ErrorInfo`]'s message relates to V1 output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum V1Message {
    /// Reproduces V1's message byte for byte.
    Verbatim(String),
    /// V1 attaches no message.
    Absent,
    /// MonoMove text used when V1's message cannot be reproduced. Not comparable
    /// with V1.
    MonoText(String),
}

impl V1Message {
    /// The text to render, if any.
    pub fn text(&self) -> Option<&str> {
        match self {
            V1Message::Verbatim(message) | V1Message::MonoText(message) => Some(message),
            V1Message::Absent => None,
        }
    }

    /// Returns `true` when this value can be compared with V1 output. `Absent`
    /// is comparable because it records that V1 produced no message.
    pub fn is_comparable(&self) -> bool {
        match self {
            V1Message::Verbatim(_) | V1Message::Absent => true,
            V1Message::MonoText(_) => false,
        }
    }
}

/// A V1 sub-status, or the reason no sub-status is available.
///
/// `Absent` means V1 produced no sub-status. `Unmodelled` means MonoMove lacks
/// the information needed to determine V1's sub-status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum V1SubStatus {
    /// V1 reports this sub-status.
    Known(u64),
    /// V1 reports none.
    Absent,
    /// MonoMove lacks the information needed to determine V1's sub-status.
    Unmodelled,
}

impl V1SubStatus {
    /// Returns the sub-status for `Known`, or `None` for `Absent` and
    /// `Unmodelled`.
    pub fn known(&self) -> Option<u64> {
        match self {
            V1SubStatus::Known(sub_status) => Some(*sub_status),
            V1SubStatus::Absent | V1SubStatus::Unmodelled => None,
        }
    }

    /// Returns `true` for a known V1 sub-status or a known absence of one.
    pub fn is_comparable(&self) -> bool {
        match self {
            V1SubStatus::Known(_) | V1SubStatus::Absent => true,
            V1SubStatus::Unmodelled => false,
        }
    }
}

/// The V1 status code, sub-status, and message for an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct V1ErrorInfo {
    pub status: StatusCode,
    pub sub_status: V1SubStatus,
    pub message: V1Message,
}

impl V1ErrorInfo {
    /// Reproduces V1's message.
    fn with_message(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            sub_status: V1SubStatus::Absent,
            message: V1Message::Verbatim(message.into()),
        }
    }

    /// A fault V1 reports without a message.
    fn with_no_message(status: StatusCode) -> Self {
        Self {
            status,
            sub_status: V1SubStatus::Absent,
            message: V1Message::Absent,
        }
    }

    /// A vector fault, which V1 reports through the sub-status alone.
    fn vector_error(sub_status: u64) -> Self {
        Self {
            status: StatusCode::VECTOR_OPERATION_ERROR,
            sub_status: V1SubStatus::Known(sub_status),
            message: V1Message::Absent,
        }
    }

    /// Uses MonoMove's message and an `Unmodelled`
    /// sub-status. MonoMove cannot reconstruct V1's message from this error, and
    /// some of these errors have a V1 sub-status that MonoMove does not record.
    fn with_mono_message(status: StatusCode, err: &impl std::fmt::Display) -> Self {
        Self {
            status,
            sub_status: V1SubStatus::Unmodelled,
            message: V1Message::MonoText(err.to_string()),
        }
    }
}

/// The result of mapping a MonoMove error to V1.
pub enum V1Equivalent {
    Described(V1ErrorInfo),
    /// V1 does not fail here: it either runs the input successfully or does not
    /// check for this at all.
    NoV1Failure,
    /// The V1 status is unknown because the error is unmapped or lacks the
    /// call-site context needed to determine the status.
    V1StatusUnknown,
}

/// Maps `err` to its V1 equivalent.
///
/// Only the three types matched below have one. That is intentional: the loader
/// runs V1's verification before specializing, so any error after that point is
/// an invariant violation on a module V1 would have run. Mapping those to a V1
/// status would hide MonoMove bugs behind a plausible V1 failure.
///
/// The other two variants fall back to a status derived from
/// [`mono_move_core::ExecutionErrorKind`] in production. Tests must not count
/// either as agreeing with V1.
pub fn describe(err: &VMInternalError) -> V1Equivalent {
    if let Some(err) = err.downcast_ref::<RuntimeError>() {
        return describe_runtime_error(err);
    }
    if let Some(err) = err.downcast_ref::<LoaderError>() {
        return describe_loader_error(err);
    }
    if err.downcast_ref::<GasExhaustedError>().is_some() {
        // V1 raises `PartialVMError::new(OUT_OF_GAS)` without a message.
        return V1Equivalent::Described(V1ErrorInfo::with_no_message(StatusCode::OUT_OF_GAS));
    }
    V1Equivalent::V1StatusUnknown
}

/// Describes a runtime fault.
pub fn describe_runtime_error(err: &RuntimeError) -> V1Equivalent {
    use RuntimeError as E;
    V1Equivalent::Described(match err {
        // One string per operation, never mentioning the operand type: a signed
        // `Add` underflow still reports "Addition overflow".
        E::ArithmeticOverflow { op, .. }
        | E::ArithmeticUnderflow { op, .. }
        | E::ArithmeticUnderOverflow { op }
        | E::ShiftAmountOutOfRange { op, .. } => match overflow_message(*op) {
            Some(message) => V1ErrorInfo::with_message(StatusCode::ARITHMETIC_ERROR, message),
            // Only reachable if a raise site pairs an op with the wrong
            // variant. Report mono's own text rather than assert V1's.
            None => V1ErrorInfo::with_mono_message(StatusCode::ARITHMETIC_ERROR, err),
        },

        // `Mod` reports the by-zero text for its overflow case too; only `Div`
        // distinguishes the two.
        E::DivisionByZero { op: ArithOp::Mod } | E::DivisionOverflow { op: ArithOp::Mod } => {
            V1ErrorInfo::with_message(StatusCode::ARITHMETIC_ERROR, "Integer remainder by zero")
        },
        E::DivisionByZero { .. } => {
            V1ErrorInfo::with_message(StatusCode::ARITHMETIC_ERROR, "Division by zero")
        },
        E::DivisionOverflow { .. } => {
            V1ErrorInfo::with_message(StatusCode::ARITHMETIC_ERROR, "Division overflow")
        },

        E::NegateMinOverflow { .. } => {
            V1ErrorInfo::with_message(StatusCode::ARITHMETIC_ERROR, "Integer negation overflow")
        },

        E::CastOutOfRange { from, to, value } => V1ErrorInfo::with_message(
            StatusCode::ARITHMETIC_ERROR,
            cast_message(*from, *to, value),
        ),

        // Reported entirely through the sub-status; the index and length are
        // dropped.
        E::VectorIndexOutOfBounds { .. } => V1ErrorInfo::vector_error(INDEX_OUT_OF_BOUNDS),
        E::PopFromEmptyVector => V1ErrorInfo::vector_error(POP_EMPTY_VEC),
        E::VecUnpackLengthMismatch { .. } => V1ErrorInfo::vector_error(VEC_UNPACK_PARITY_MISMATCH),

        E::ResourceDoesNotExist { op, addr } => {
            V1ErrorInfo::with_message(StatusCode::MISSING_DATA, match op {
                GlobalStorageOp::MoveFrom => format!("Failed to move resource from {addr:?}"),
                GlobalStorageOp::BorrowGlobal | GlobalStorageOp::BorrowGlobalMut => {
                    format!("Failed to borrow global resource from {addr:?}")
                },
            })
        },
        E::ResourceAlreadyExists { addr } => V1ErrorInfo::with_message(
            StatusCode::RESOURCE_ALREADY_EXISTS,
            format!("Failed to move resource into {addr:?}"),
        ),

        // V1 names both variants ("expected enum variant Circle, found
        // Square"), which needs variant names at the fault site; mono carries
        // only the found tag. Reported with mono text rather than reproduced.
        E::EnumVariantMismatch { .. } => {
            V1ErrorInfo::with_mono_message(StatusCode::STRUCT_VARIANT_MISMATCH, err)
        },

        // Both statuses are Execution-range, so the transaction is kept and
        // charged, matching V1.
        E::InvalidAbortMessage { cause } => V1ErrorInfo::with_message(
            StatusCode::INVALID_ABORT_MESSAGE,
            format!("Invalid UTF-8 string: {cause}"),
        ),
        E::AbortMessageTooLong { len, max } => V1ErrorInfo::with_message(
            StatusCode::ABORT_MESSAGE_LIMIT_EXCEEDED,
            format!("Expected at most {max} bytes, got {len} bytes"),
        ),

        E::StackOverflow => V1ErrorInfo::with_no_message(StatusCode::CALL_STACK_OVERFLOW),
        E::OutOfHeapMemory { .. } | E::AllocationTooLarge { .. } | E::VecAllocSizeOverflow => {
            V1ErrorInfo::with_no_message(StatusCode::MEMORY_LIMIT_EXCEEDED)
        },

        E::StateKeyTypeTooDeep => {
            V1ErrorInfo::with_mono_message(StatusCode::VALUE_SERIALIZATION_ERROR, err)
        },

        // The same deserializer serves entry arguments, storage reads, constant
        // pools, and natives; V1 reports each differently, and this error does
        // not record which it came from.
        E::BCSEof
        | E::BCSInvalidUleb
        | E::BCSSequenceTooLong { .. }
        | E::BCSRemainingInput { .. }
        | E::BCSInvalidBool { .. }
        | E::BCSSignerNotDeserializable => return V1Equivalent::V1StatusUnknown,

        // A feature V1 has and MonoMove does not, so V1 runs the input.
        E::Unsupported(_) => return V1Equivalent::NoV1Failure,

        E::InvariantViolation(_) | E::ResourceProvider(_) => {
            V1ErrorInfo::with_mono_message(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR, err)
        },
    })
}

/// Describes a loader fault.
fn describe_loader_error(err: &LoaderError) -> V1Equivalent {
    use LoaderError as L;
    let info = match err {
        // The address is hex without a `0x` prefix, as V1 renders it.
        L::ModuleNotFound { address, name } => V1ErrorInfo::with_message(
            StatusCode::LINKER_ERROR,
            format!(
                "Linker Error: Module {}::{} doesn't exist",
                address.to_hex(),
                name
            ),
        ),
        // A stale function value can name a function that no longer resolves.
        L::FunctionNotFound { .. } => {
            V1ErrorInfo::with_mono_message(StatusCode::FUNCTION_RESOLUTION_FAILURE, err)
        },
        // MonoMove-only loading or lowering gaps have no corresponding V1
        // failure and therefore map to `NoV1Failure`.
        //
        // TODO(correctness): `NativeFunctionNotLoadable` is ambiguous. If V1
        // also lacks the declared native's implementation, V1 reports
        // `MISSING_DEPENDENCY` at the native definition while this mapping
        // reports no V1 failure. Distinguishing the cases requires V1's native
        // table. Because only special addresses may publish native declarations,
        // this can occur only if a framework release declares a native without
        // registering its implementation.
        L::NativeFunctionNotLoadable { .. } | L::LoweringSkipped { .. } => {
            return V1Equivalent::NoV1Failure
        },
        L::GlobalContext(_) | L::InvariantViolation(_) => {
            V1ErrorInfo::with_mono_message(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR, err)
        },
    };
    V1Equivalent::Described(info)
}

/// V1's overflow message for an add, subtract, multiply, or shift.
///
/// `None` for the other operations: they either report their own message or
/// cannot overflow.
fn overflow_message(op: ArithOp) -> Option<&'static str> {
    match op {
        ArithOp::Add => Some("Addition overflow"),
        ArithOp::Sub => Some("Subtraction overflow"),
        ArithOp::Mul => Some("Multiplication overflow"),
        ArithOp::Shl => Some("Shift Left overflow"),
        ArithOp::Shr => Some("Shift Right overflow"),
        ArithOp::Div
        | ArithOp::Mod
        | ArithOp::Negate
        | ArithOp::BitAnd
        | ArithOp::BitOr
        | ArithOp::BitXor => None,
    }
}

/// Reproduces V1's cast message. Its type names are Rust names, not Move ones,
/// so `IntTy`'s own display cannot be used.
fn cast_message(from: IntTy, to: IntTy, value: &ReportedIntValue) -> String {
    format!(
        "Cannot cast {}({}) to {}",
        cast_source_name(from, to),
        value,
        cast_target_name(to)
    )
}

/// The source type's name. The 256-bit names are unqualified except in the one
/// V1 arm that spells the source as a path, `u256` -> `i256`.
fn cast_source_name(from: IntTy, to: IntTy) -> &'static str {
    match (from, to) {
        (IntTy::U256, IntTy::I256) => "int256::U256",
        (IntTy::U256, _) => "U256",
        (IntTy::I256, _) => "I256",
        _ => primitive_name(from),
    }
}

/// The target type's name; 256-bit targets are always spelled as paths.
fn cast_target_name(to: IntTy) -> &'static str {
    match to {
        IntTy::U256 => "int256::U256",
        IntTy::I256 => "int256::I256",
        _ => primitive_name(to),
    }
}

/// The Rust primitive name for the fixed-width types.
fn primitive_name(ty: IntTy) -> &'static str {
    match ty {
        IntTy::U8 => "u8",
        IntTy::U16 => "u16",
        IntTy::U32 => "u32",
        IntTy::U64 => "u64",
        IntTy::U128 => "u128",
        IntTy::I8 => "i8",
        IntTy::I16 => "i16",
        IntTy::I32 => "i32",
        IntTy::I64 => "i64",
        IntTy::I128 => "i128",
        // Unreachable: the 256-bit spelling depends on the position, so callers
        // resolve it before delegating here.
        IntTy::U256 => "U256",
        IntTy::I256 => "I256",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::{account_address::AccountAddress, int256::I256};

    /// The mapping for an error V1 has a counterpart for.
    fn described(err: &RuntimeError) -> V1ErrorInfo {
        match describe_runtime_error(err) {
            V1Equivalent::Described(info) => info,
            _ => panic!("expected a V1 counterpart for {err}"),
        }
    }

    fn message_of(err: &RuntimeError) -> String {
        described(err)
            .message
            .text()
            .expect("this error carries a V1 message")
            .to_string()
    }

    #[test]
    fn arithmetic_messages_match_v1() {
        let cases = [
            (
                RuntimeError::ArithmeticUnderOverflow { op: ArithOp::Add },
                "Addition overflow",
            ),
            (
                RuntimeError::ArithmeticUnderOverflow { op: ArithOp::Sub },
                "Subtraction overflow",
            ),
            (
                RuntimeError::ArithmeticUnderOverflow { op: ArithOp::Mul },
                "Multiplication overflow",
            ),
            (
                RuntimeError::DivisionByZero { op: ArithOp::Div },
                "Division by zero",
            ),
            (
                RuntimeError::DivisionOverflow { op: ArithOp::Div },
                "Division overflow",
            ),
            (
                RuntimeError::DivisionByZero { op: ArithOp::Mod },
                "Integer remainder by zero",
            ),
            (
                RuntimeError::NegateMinOverflow { ty: IntTy::I64 },
                "Integer negation overflow",
            ),
            (
                RuntimeError::ShiftAmountOutOfRange {
                    op: ArithOp::Shl,
                    ty: IntTy::U64,
                    shift_amount: 64,
                    bit_width: 64,
                },
                "Shift Left overflow",
            ),
            (
                RuntimeError::ShiftAmountOutOfRange {
                    op: ArithOp::Shr,
                    ty: IntTy::U64,
                    shift_amount: 64,
                    bit_width: 64,
                },
                "Shift Right overflow",
            ),
        ];
        for (err, expected) in cases {
            assert_eq!(message_of(&err), expected);
            let info = described(&err);
            assert_eq!(info.status, StatusCode::ARITHMETIC_ERROR);
            assert_eq!(info.sub_status, V1SubStatus::Absent);
        }
    }

    /// The type-carrying variants must describe identically to the type-less
    /// ones, since V1's message ignores the operand type.
    #[test]
    fn specialized_arithmetic_matches_polymorphic() {
        assert_eq!(
            message_of(&RuntimeError::ArithmeticOverflow {
                op: ArithOp::Add,
                ty: IntTy::U64
            }),
            "Addition overflow"
        );
        assert_eq!(
            message_of(&RuntimeError::ArithmeticUnderflow {
                op: ArithOp::Sub,
                ty: IntTy::U64
            }),
            "Subtraction overflow"
        );
    }

    /// `Mod` reports the by-zero text even for `MIN % -1`.
    #[test]
    fn remainder_overflow_reports_by_zero_text() {
        assert_eq!(
            message_of(&RuntimeError::DivisionOverflow { op: ArithOp::Mod }),
            "Integer remainder by zero"
        );
    }

    /// Cast messages embed the operand and use Rust type names, whose 256-bit
    /// spelling differs between the source and target positions.
    #[test]
    fn cast_messages_match_v1() {
        let cast = |from, to, value: ReportedIntValue| {
            message_of(&RuntimeError::CastOutOfRange { from, to, value })
        };
        assert_eq!(
            cast(IntTy::I8, IntTy::U8, ReportedIntValue::from(-1i8)),
            "Cannot cast i8(-1) to u8"
        );
        assert_eq!(
            cast(
                IntTy::U64,
                IntTy::I64,
                ReportedIntValue::from(9223372036854775808u64)
            ),
            "Cannot cast u64(9223372036854775808) to i64"
        );
        assert_eq!(
            cast(
                IntTy::I256,
                IntTy::U256,
                ReportedIntValue::from(I256::from(-1i8))
            ),
            "Cannot cast I256(-1) to int256::U256"
        );
        assert_eq!(
            cast(IntTy::U256, IntTy::I256, ReportedIntValue::from(2u8)),
            "Cannot cast int256::U256(2) to int256::I256"
        );
    }

    /// Addresses render as padded hex, not the `0x1` short form.
    #[test]
    fn global_storage_messages_match_v1() {
        let addr = AccountAddress::ONE;
        let expected_addr = "0000000000000000000000000000000000000000000000000000000000000001";
        assert_eq!(
            message_of(&RuntimeError::ResourceDoesNotExist {
                op: GlobalStorageOp::BorrowGlobal,
                addr
            }),
            format!("Failed to borrow global resource from {expected_addr}")
        );
        assert_eq!(
            message_of(&RuntimeError::ResourceDoesNotExist {
                op: GlobalStorageOp::MoveFrom,
                addr
            }),
            format!("Failed to move resource from {expected_addr}")
        );
        let already_exists = RuntimeError::ResourceAlreadyExists { addr };
        assert_eq!(
            message_of(&already_exists),
            format!("Failed to move resource into {expected_addr}")
        );
        assert_eq!(
            described(&already_exists).status,
            StatusCode::RESOURCE_ALREADY_EXISTS
        );
    }

    /// Vector faults report through the sub-status and carry no message.
    #[test]
    fn vector_errors_describe_by_sub_status() {
        let cases = [
            (
                RuntimeError::VectorIndexOutOfBounds {
                    op: mono_move_runtime::VecOp::Borrow,
                    idx: 3,
                    len: 1,
                },
                INDEX_OUT_OF_BOUNDS,
            ),
            (RuntimeError::PopFromEmptyVector, POP_EMPTY_VEC),
            (
                RuntimeError::VecUnpackLengthMismatch {
                    expected: 2,
                    actual: 3,
                },
                VEC_UNPACK_PARITY_MISMATCH,
            ),
        ];
        for (err, expected_sub_status) in cases {
            let info = described(&err);
            assert_eq!(info.status, StatusCode::VECTOR_OPERATION_ERROR);
            assert_eq!(info.sub_status, V1SubStatus::Known(expected_sub_status));
            assert_eq!(info.message, V1Message::Absent);
        }
    }

    /// Errors that V1 reports without a message remain message-free.
    #[test]
    fn message_less_errors_stay_message_less() {
        for (err, expected_status) in [
            (RuntimeError::StackOverflow, StatusCode::CALL_STACK_OVERFLOW),
            (
                RuntimeError::OutOfHeapMemory { requested: 64 },
                StatusCode::MEMORY_LIMIT_EXCEEDED,
            ),
        ] {
            let info = described(&err);
            assert_eq!(info.status, expected_status);
            assert_eq!(info.message, V1Message::Absent);
        }
        let gas = VMInternalError::new(GasExhaustedError);
        let V1Equivalent::Described(info) = describe(&gas) else {
            panic!("gas exhaustion has a V1 counterpart");
        };
        assert_eq!(info.status, StatusCode::OUT_OF_GAS);
        assert_eq!(info.message, V1Message::Absent);
    }

    #[test]
    fn abort_message_errors_match_v1() {
        let cause = String::from_utf8(vec![0xFF])
            .expect_err("0xff is not valid UTF-8")
            .utf8_error();
        let invalid = RuntimeError::InvalidAbortMessage { cause };
        assert_eq!(
            message_of(&invalid),
            "Invalid UTF-8 string: invalid utf-8 sequence of 1 bytes from index 0"
        );
        assert_eq!(
            described(&invalid).status,
            StatusCode::INVALID_ABORT_MESSAGE
        );

        let too_long = RuntimeError::AbortMessageTooLong {
            len: 1025,
            max: 1024,
        };
        assert_eq!(
            message_of(&too_long),
            "Expected at most 1024 bytes, got 1025 bytes"
        );
        assert_eq!(
            described(&too_long).status,
            StatusCode::ABORT_MESSAGE_LIMIT_EXCEEDED
        );
    }

    #[test]
    fn module_not_found_matches_v1_linker_message() {
        let V1Equivalent::Described(info) = describe_loader_error(&LoaderError::ModuleNotFound {
            address: AccountAddress::from_hex_literal("0x99").expect("valid address"),
            name: "test_enum".to_string(),
        }) else {
            panic!("a missing module is a V1 linker error");
        };
        assert_eq!(info.status, StatusCode::LINKER_ERROR);
        assert_eq!(
            info.message.text().expect("linker errors carry a message"),
            "Linker Error: Module \
             0000000000000000000000000000000000000000000000000000000000000099::test_enum \
             doesn't exist"
        );
    }

    #[test]
    fn v2_only_faults_report_no_v1_failure() {
        assert!(matches!(
            describe_loader_error(&LoaderError::LoweringSkipped { reason: "nominal" }),
            V1Equivalent::NoV1Failure
        ));
        assert!(matches!(
            describe_loader_error(&LoaderError::NativeFunctionNotLoadable {
                address: AccountAddress::ONE,
                module: "vector".to_string(),
                name: "length".to_string(),
            }),
            V1Equivalent::NoV1Failure
        ));
    }

    #[test]
    fn ambiguous_deserialization_errors_leave_the_v1_status_unknown() {
        assert!(matches!(
            describe_runtime_error(&RuntimeError::BCSEof),
            V1Equivalent::V1StatusUnknown
        ));
    }

    /// Messages that cannot reproduce V1 are excluded from message comparisons.
    #[test]
    fn unreproducible_messages_use_mono_text() {
        let mono_text = described(&RuntimeError::EnumVariantMismatch { tag: 2 });
        assert_eq!(mono_text.status, StatusCode::STRUCT_VARIANT_MISMATCH);
        assert!(!mono_text.message.is_comparable());
        assert!(matches!(mono_text.message, V1Message::MonoText(_)));

        // A reproduced message, and the absence of one, are both comparable.
        let verbatim = described(&RuntimeError::DivisionByZero { op: ArithOp::Div });
        assert!(verbatim.message.is_comparable());
        assert!(described(&RuntimeError::StackOverflow)
            .message
            .is_comparable());
    }

    #[test]
    fn describe_downcasts_each_subsystem() {
        let runtime = VMInternalError::new(RuntimeError::DivisionByZero { op: ArithOp::Div });
        let V1Equivalent::Described(runtime) = describe(&runtime) else {
            panic!("runtime errors are mapped");
        };
        assert_eq!(runtime.status, StatusCode::ARITHMETIC_ERROR);

        let loader = VMInternalError::new(LoaderError::ModuleNotFound {
            address: AccountAddress::ONE,
            name: "missing".to_string(),
        });
        let V1Equivalent::Described(loader) = describe(&loader) else {
            panic!("loader errors are mapped");
        };
        assert_eq!(loader.status, StatusCode::LINKER_ERROR);
    }
}
