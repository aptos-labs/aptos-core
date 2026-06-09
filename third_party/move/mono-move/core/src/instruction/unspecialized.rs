// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Operand types for the unspecialized integer micro-ops in
//! [`super::MicroOp`].
//!
//! ## Principle of specialization
//!
//! A given `(op, type)` combination gets its own dedicated [`super::MicroOp`]
//! variant only when there is a measured benefit to specializing it
//! (e.g., u64 arithmetic is the dominant case). Everything else goes through
//! the unspecialized per-kind variants defined here, which tag-dispatch on
//! the operand type at runtime.

use super::{CodeOffset, FrameOffset};
use crate::types::{InternedType, Type};
use move_core_types::int256::{I256, U256};
use std::fmt;

/// Move's integer types. Carries the type tag for the unspecialized
/// micro-ops ([`super::MicroOp::IntShl`], [`super::MicroOp::IntNegate`], …)
/// and as a building block for future cast / convert operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IntTy {
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    I8,
    I16,
    I32,
    I64,
    I128,
    I256,
}

impl IntTy {
    /// Width of the corresponding slot, in bytes.
    #[inline(always)]
    pub const fn byte_width(self) -> usize {
        match self {
            IntTy::U8 | IntTy::I8 => 1,
            IntTy::U16 | IntTy::I16 => 2,
            IntTy::U32 | IntTy::I32 => 4,
            IntTy::U64 | IntTy::I64 => 8,
            IntTy::U128 | IntTy::I128 => 16,
            IntTy::U256 | IntTy::I256 => 32,
        }
    }

    #[inline(always)]
    pub const fn bit_width(self) -> usize {
        self.byte_width() * 8
    }

    #[inline(always)]
    pub const fn is_signed(self) -> bool {
        matches!(
            self,
            IntTy::I8 | IntTy::I16 | IntTy::I32 | IntTy::I64 | IntTy::I128 | IntTy::I256
        )
    }

    pub fn from_type(ty: &Type) -> Option<Self> {
        Some(match ty {
            Type::U8 => IntTy::U8,
            Type::U16 => IntTy::U16,
            Type::U32 => IntTy::U32,
            Type::U64 => IntTy::U64,
            Type::U128 => IntTy::U128,
            Type::U256 => IntTy::U256,
            Type::I8 => IntTy::I8,
            Type::I16 => IntTy::I16,
            Type::I32 => IntTy::I32,
            Type::I64 => IntTy::I64,
            Type::I128 => IntTy::I128,
            Type::I256 => IntTy::I256,
            _ => return None,
        })
    }
}

impl fmt::Display for IntTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            IntTy::U8 => "u8",
            IntTy::U16 => "u16",
            IntTy::U32 => "u32",
            IntTy::U64 => "u64",
            IntTy::U128 => "u128",
            IntTy::U256 => "u256",
            IntTy::I8 => "i8",
            IntTy::I16 => "i16",
            IntTy::I32 => "i32",
            IntTy::I64 => "i64",
            IntTy::I128 => "i128",
            IntTy::I256 => "i256",
        })
    }
}

/// An integer operand: either a slot reference or an inline immediate.
///
/// The variant determines both the operand's integer type and whether the
/// value lives in a frame slot or is baked into the op.
///
/// # Layout
///
/// The narrow imm arms (u8 / u16 / u32 / u64, i8 / i16 / i32 / i64) store
/// their value inline. The four wide arms (u128 / u256 / i128 / i256) box
/// their payload so the enum stays at 16 bytes on 64-bit hosts — the
/// alternative would inflate every micro-op that carries an `IntOperand`
/// to fit a 32-byte u256.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntOperand {
    SlotU8(FrameOffset),
    SlotU16(FrameOffset),
    SlotU32(FrameOffset),
    SlotU64(FrameOffset),
    SlotU128(FrameOffset),
    SlotU256(FrameOffset),
    SlotI8(FrameOffset),
    SlotI16(FrameOffset),
    SlotI32(FrameOffset),
    SlotI64(FrameOffset),
    SlotI128(FrameOffset),
    SlotI256(FrameOffset),
    ImmU8(u8),
    ImmU16(u16),
    ImmU32(u32),
    ImmU64(u64),
    ImmI8(i8),
    ImmI16(i16),
    ImmI32(i32),
    ImmI64(i64),
    ImmU128(Box<u128>),
    ImmU256(Box<U256>),
    ImmI128(Box<i128>),
    ImmI256(Box<I256>),
}

const _: () = assert!(std::mem::size_of::<IntOperand>() == 16);

impl IntOperand {
    /// Width of the operand in bytes.
    #[inline(always)]
    pub fn byte_width(&self) -> usize {
        use IntOperand::*;
        match self {
            SlotU8(_) | ImmU8(_) | SlotI8(_) | ImmI8(_) => 1,
            SlotU16(_) | ImmU16(_) | SlotI16(_) | ImmI16(_) => 2,
            SlotU32(_) | ImmU32(_) | SlotI32(_) | ImmI32(_) => 4,
            SlotU64(_) | ImmU64(_) | SlotI64(_) | ImmI64(_) => 8,
            SlotU128(_) | ImmU128(_) | SlotI128(_) | ImmI128(_) => 16,
            SlotU256(_) | ImmU256(_) | SlotI256(_) | ImmI256(_) => 32,
        }
    }

    #[inline(always)]
    pub fn is_signed(&self) -> bool {
        use IntOperand::*;
        matches!(
            self,
            SlotI8(_)
                | SlotI16(_)
                | SlotI32(_)
                | SlotI64(_)
                | SlotI128(_)
                | SlotI256(_)
                | ImmI8(_)
                | ImmI16(_)
                | ImmI32(_)
                | ImmI64(_)
                | ImmI128(_)
                | ImmI256(_)
        )
    }

    /// `Some(offset)` for the slot arms; `None` for the imm arms.
    #[inline(always)]
    pub fn slot_offset(&self) -> Option<FrameOffset> {
        use IntOperand::*;
        match self {
            SlotU8(o) | SlotU16(o) | SlotU32(o) | SlotU64(o) | SlotU128(o) | SlotU256(o)
            | SlotI8(o) | SlotI16(o) | SlotI32(o) | SlotI64(o) | SlotI128(o) | SlotI256(o) => {
                Some(*o)
            },
            ImmU8(_) | ImmU16(_) | ImmU32(_) | ImmU64(_) | ImmU128(_) | ImmU256(_) | ImmI8(_)
            | ImmI16(_) | ImmI32(_) | ImmI64(_) | ImmI128(_) | ImmI256(_) => None,
        }
    }

    /// True iff this operand is an immediate equal to zero.
    #[inline(always)]
    pub fn is_zero_imm(&self) -> bool {
        use IntOperand::*;
        match self {
            ImmU8(v) => *v == 0,
            ImmU16(v) => *v == 0,
            ImmU32(v) => *v == 0,
            ImmU64(v) => *v == 0,
            ImmI8(v) => *v == 0,
            ImmI16(v) => *v == 0,
            ImmI32(v) => *v == 0,
            ImmI64(v) => *v == 0,
            ImmU128(b) => **b == 0,
            ImmI128(b) => **b == 0,
            ImmU256(b) => **b == U256::ZERO,
            ImmI256(b) => **b == I256::ZERO,
            SlotU8(_) | SlotU16(_) | SlotU32(_) | SlotU64(_) | SlotU128(_) | SlotU256(_)
            | SlotI8(_) | SlotI16(_) | SlotI32(_) | SlotI64(_) | SlotI128(_) | SlotI256(_) => false,
        }
    }

    /// Build the slot arm matching `ty`.
    pub fn slot(ty: IntTy, off: FrameOffset) -> Self {
        match ty {
            IntTy::U8 => IntOperand::SlotU8(off),
            IntTy::U16 => IntOperand::SlotU16(off),
            IntTy::U32 => IntOperand::SlotU32(off),
            IntTy::U64 => IntOperand::SlotU64(off),
            IntTy::U128 => IntOperand::SlotU128(off),
            IntTy::U256 => IntOperand::SlotU256(off),
            IntTy::I8 => IntOperand::SlotI8(off),
            IntTy::I16 => IntOperand::SlotI16(off),
            IntTy::I32 => IntOperand::SlotI32(off),
            IntTy::I64 => IntOperand::SlotI64(off),
            IntTy::I128 => IntOperand::SlotI128(off),
            IntTy::I256 => IntOperand::SlotI256(off),
        }
    }
}

impl fmt::Display for IntOperand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use IntOperand::*;
        match self {
            SlotU8(o) => write!(f, "u8 [{}]", o.0),
            SlotU16(o) => write!(f, "u16 [{}]", o.0),
            SlotU32(o) => write!(f, "u32 [{}]", o.0),
            SlotU64(o) => write!(f, "u64 [{}]", o.0),
            SlotU128(o) => write!(f, "u128 [{}]", o.0),
            SlotU256(o) => write!(f, "u256 [{}]", o.0),
            SlotI8(o) => write!(f, "i8 [{}]", o.0),
            SlotI16(o) => write!(f, "i16 [{}]", o.0),
            SlotI32(o) => write!(f, "i32 [{}]", o.0),
            SlotI64(o) => write!(f, "i64 [{}]", o.0),
            SlotI128(o) => write!(f, "i128 [{}]", o.0),
            SlotI256(o) => write!(f, "i256 [{}]", o.0),
            ImmU8(v) => write!(f, "u8 #{}", v),
            ImmU16(v) => write!(f, "u16 #{}", v),
            ImmU32(v) => write!(f, "u32 #{}", v),
            ImmU64(v) => write!(f, "u64 #{}", v),
            ImmI8(v) => write!(f, "i8 #{}", v),
            ImmI16(v) => write!(f, "i16 #{}", v),
            ImmI32(v) => write!(f, "i32 #{}", v),
            ImmI64(v) => write!(f, "i64 #{}", v),
            ImmU128(b) => write!(f, "u128 #{}", **b),
            ImmI128(b) => write!(f, "i128 #{}", **b),
            ImmU256(b) => write!(f, "u256 #{}", **b),
            ImmI256(b) => write!(f, "i256 #{}", **b),
        }
    }
}

/// `dst = lhs <kind> rhs`. The kind comes from the wrapping
/// [`super::MicroOp`] variant. `lhs` is a slot whose width matches
/// `rhs.byte_width()`; `dst` is a slot of the same width.
///
/// Invariants (enforced at construction or by the verifier):
///   - `dst`, `lhs`, and the slot of `rhs` (if `rhs` is a slot) point at
///     in-bounds frame regions of width `rhs.byte_width()`.
///   - For bitwise kinds (BitAnd / BitOr / BitXor), `rhs` is unsigned.
///
/// TODO: consider reverse-imm variants (e.g. `dst = imm - lhs` for `Sub`,
/// and equivalents for `Div`/`Mod`) — non-commutative ops can't express
/// `imm op slot` with the current shape and instead need a separate slot
/// load + binop, which is one extra micro-op per such case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntBinaryOp {
    pub dst: FrameOffset,
    pub lhs: FrameOffset,
    pub rhs: IntOperand,
}

/// Shift amount for [`IntShiftOp`]. Always `u8` in Move.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftOperand {
    SlotU8(FrameOffset),
    ImmU8(u8),
}

impl fmt::Display for ShiftOperand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShiftOperand::SlotU8(o) => write!(f, "[{}]", o.0),
            ShiftOperand::ImmU8(v) => write!(f, "#{}", v),
        }
    }
}

/// `dst = lhs <direction> rhs`. Aborts if `rhs >= ty.bit_width()`.
/// Invariant: `ty` is unsigned (verifier-enforced).
///
/// TODO: consider folding into [`IntBinaryOp`] (letting [`IntOperand`]
/// carry the always-`u8` shift amount). Kept separate today since shifts
/// don't share the binary dispatch path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntShiftOp {
    pub ty: IntTy,
    pub dst: FrameOffset,
    pub lhs: FrameOffset,
    pub rhs: ShiftOperand,
}

/// `dst = -src`. Aborts when `src == ty::MIN`.
/// Invariant: `ty` is signed (verifier-enforced).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntNegateOp {
    pub ty: IntTy,
    pub dst: FrameOffset,
    pub src: FrameOffset,
}

/// Universal cast operation, supporting all integer pairs — including
/// self-casts.
///
/// Abort semantics follow one universal rule: the cast succeeds iff the
/// source value fits in the target type's range, and aborts otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntCastOp {
    pub from: IntTy,
    pub to: IntTy,
    pub dst: FrameOffset,
    pub src: FrameOffset,
}

/// Comparison relation for [`super::MicroOp::IntCmp`]. `CmpKind` itself
/// carries no type or width: the operands determine both, so one variant is
/// reused across every type it applies to.
///
/// - Ordering variants (`Lt`/`Le`/`Gt`/`Ge`) apply to integer operands only;
///   the operand's type decides whether the comparison is signed or unsigned.
/// - Equality variants (`Eq`/`Neq`) apply to any flat value of any width —
///   every integer width plus `bool` (1 byte) and `address` (32 bytes) —
///   since they just compare bit patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CmpKind {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Neq,
}

impl fmt::Display for CmpKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            CmpKind::Lt => "<",
            CmpKind::Le => "<=",
            CmpKind::Gt => ">",
            CmpKind::Ge => ">=",
            CmpKind::Eq => "==",
            CmpKind::Neq => "!=",
        })
    }
}

/// `dst = (lhs <op> rhs)` producing a 1-byte boolean (`0` / `1`).
/// `lhs` is an integer slot (of the same type as `rhs`).
/// `dst` is a 1-byte slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntCmpOp {
    pub op: CmpKind,
    pub dst: FrameOffset,
    pub lhs: FrameOffset,
    pub rhs: IntOperand,
}

/// Fused compare-and-branch: jump to `target` if `op(lhs, rhs)` holds. Like
/// [`IntCmpOp`] but branches on the result instead of storing it.
/// Note that there are specialized `*U64` variants of this micro-op.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpIntCmpOp {
    pub target: CodeOffset,
    pub op: CmpKind,
    pub lhs: FrameOffset,
    pub rhs: IntOperand,
    pub gas_taken: u64,
    pub gas_fallthrough: u64,
}

/// `dst = (lhs == rhs)` (or `!=` when `negate`), a structural equality over
/// the aggregate values at `lhs`/`rhs`, producing a 1-byte boolean. Used for
/// aggregate types (vectors, structs) — everything that is not a flat scalar
/// handled by [`IntCmpOp`]. A vector slot holds a pointer to its heap data,
/// which the comparison reads through.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ValueCmpOp {
    pub negate: bool,
    pub dst: FrameOffset,
    pub lhs: FrameOffset,
    pub rhs: FrameOffset,
    pub ty: InternedType,
}

/// Like [`ValueCmpOp`], but the operands are **references**: `lhs`/`rhs` hold
/// 16-byte fat pointers, read through to obtain the operand pointers of the
/// referent value of type `ty`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ValueRefCmpOp {
    pub negate: bool,
    pub dst: FrameOffset,
    pub lhs: FrameOffset,
    pub rhs: FrameOffset,
    pub ty: InternedType,
}

/// Fused compare-and-branch counterpart of [`ValueCmpOp`]: jump to `target`
/// if the equality result holds (negated when `negate`). Operands are inline
/// values; see [`ValueCmpOp`] for the field meanings.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct JumpValueCmpOp {
    pub target: CodeOffset,
    pub negate: bool,
    pub lhs: FrameOffset,
    pub rhs: FrameOffset,
    pub ty: InternedType,
    pub gas_taken: u64,
    pub gas_fallthrough: u64,
}

/// Fused compare-and-branch counterpart of [`ValueRefCmpOp`]: operands are
/// 16-byte fat pointers read through to the referent values.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct JumpValueRefCmpOp {
    pub target: CodeOffset,
    pub negate: bool,
    pub lhs: FrameOffset,
    pub rhs: FrameOffset,
    pub ty: InternedType,
    pub gas_taken: u64,
    pub gas_fallthrough: u64,
}
