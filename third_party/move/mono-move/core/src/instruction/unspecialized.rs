// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Operand types for the unspecialized integer micro-ops in
//! [`super::MicroOp`] (`IntAdd`/`IntSub`/…/`IntShl`/`IntShr`/`IntNegate`).

use super::FrameOffset;
use crate::types::Type;
use move_core_types::int256::{I256, U256};
use std::fmt;

/// Unsigned non-u64 integer widths. Used by [`super::MicroOp::IntShl`] /
/// [`super::MicroOp::IntShr`] (Move shifts are unsigned-only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UnspecializedUnsignedIntTy {
    U8,
    U16,
    U32,
    U128,
    U256,
}

impl UnspecializedUnsignedIntTy {
    #[inline(always)]
    pub const fn byte_width(self) -> u32 {
        match self {
            UnspecializedUnsignedIntTy::U8 => 1,
            UnspecializedUnsignedIntTy::U16 => 2,
            UnspecializedUnsignedIntTy::U32 => 4,
            UnspecializedUnsignedIntTy::U128 => 16,
            UnspecializedUnsignedIntTy::U256 => 32,
        }
    }

    #[inline(always)]
    pub const fn bit_width(self) -> u32 {
        self.byte_width() * 8
    }

    pub fn from_type(ty: &Type) -> Option<Self> {
        Some(match ty {
            Type::U8 => UnspecializedUnsignedIntTy::U8,
            Type::U16 => UnspecializedUnsignedIntTy::U16,
            Type::U32 => UnspecializedUnsignedIntTy::U32,
            Type::U128 => UnspecializedUnsignedIntTy::U128,
            Type::U256 => UnspecializedUnsignedIntTy::U256,
            _ => return None,
        })
    }
}

impl fmt::Display for UnspecializedUnsignedIntTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            UnspecializedUnsignedIntTy::U8 => "u8",
            UnspecializedUnsignedIntTy::U16 => "u16",
            UnspecializedUnsignedIntTy::U32 => "u32",
            UnspecializedUnsignedIntTy::U128 => "u128",
            UnspecializedUnsignedIntTy::U256 => "u256",
        })
    }
}

/// Signed integer widths. Used by [`super::MicroOp::IntNegate`] (Move
/// negate is signed-only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UnspecializedSignedIntTy {
    I8,
    I16,
    I32,
    I64,
    I128,
    I256,
}

impl UnspecializedSignedIntTy {
    #[inline(always)]
    pub const fn byte_width(self) -> u32 {
        match self {
            UnspecializedSignedIntTy::I8 => 1,
            UnspecializedSignedIntTy::I16 => 2,
            UnspecializedSignedIntTy::I32 => 4,
            UnspecializedSignedIntTy::I64 => 8,
            UnspecializedSignedIntTy::I128 => 16,
            UnspecializedSignedIntTy::I256 => 32,
        }
    }

    #[inline(always)]
    pub const fn bit_width(self) -> u32 {
        self.byte_width() * 8
    }

    pub fn from_type(ty: &Type) -> Option<Self> {
        Some(match ty {
            Type::I8 => UnspecializedSignedIntTy::I8,
            Type::I16 => UnspecializedSignedIntTy::I16,
            Type::I32 => UnspecializedSignedIntTy::I32,
            Type::I64 => UnspecializedSignedIntTy::I64,
            Type::I128 => UnspecializedSignedIntTy::I128,
            Type::I256 => UnspecializedSignedIntTy::I256,
            _ => return None,
        })
    }
}

impl fmt::Display for UnspecializedSignedIntTy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            UnspecializedSignedIntTy::I8 => "i8",
            UnspecializedSignedIntTy::I16 => "i16",
            UnspecializedSignedIntTy::I32 => "i32",
            UnspecializedSignedIntTy::I64 => "i64",
            UnspecializedSignedIntTy::I128 => "i128",
            UnspecializedSignedIntTy::I256 => "i256",
        })
    }
}

/// The rhs of [`IntBinaryOp`]. The variant determines both the operand's
/// integer type and whether the value lives in a frame slot (`Reg*`) or is
/// baked into the op (`Imm*`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntOperand {
    RegU8(FrameOffset),
    RegU16(FrameOffset),
    RegU32(FrameOffset),
    RegU64(FrameOffset),
    RegU128(FrameOffset),
    RegU256(FrameOffset),
    RegI8(FrameOffset),
    RegI16(FrameOffset),
    RegI32(FrameOffset),
    RegI64(FrameOffset),
    RegI128(FrameOffset),
    RegI256(FrameOffset),
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

impl IntOperand {
    /// Width of the operand in bytes.
    #[inline(always)]
    pub fn byte_width(&self) -> u32 {
        use IntOperand::*;
        match self {
            RegU8(_) | ImmU8(_) | RegI8(_) | ImmI8(_) => 1,
            RegU16(_) | ImmU16(_) | RegI16(_) | ImmI16(_) => 2,
            RegU32(_) | ImmU32(_) | RegI32(_) | ImmI32(_) => 4,
            RegU64(_) | ImmU64(_) | RegI64(_) | ImmI64(_) => 8,
            RegU128(_) | ImmU128(_) | RegI128(_) | ImmI128(_) => 16,
            RegU256(_) | ImmU256(_) | RegI256(_) | ImmI256(_) => 32,
        }
    }

    #[inline(always)]
    pub fn is_signed(&self) -> bool {
        use IntOperand::*;
        matches!(
            self,
            RegI8(_)
                | RegI16(_)
                | RegI32(_)
                | RegI64(_)
                | RegI128(_)
                | RegI256(_)
                | ImmI8(_)
                | ImmI16(_)
                | ImmI32(_)
                | ImmI64(_)
                | ImmI128(_)
                | ImmI256(_)
        )
    }

    /// `Some(offset)` for the Reg arms; `None` for the imm arms.
    #[inline(always)]
    pub fn reg_offset(&self) -> Option<FrameOffset> {
        use IntOperand::*;
        match self {
            RegU8(o) | RegU16(o) | RegU32(o) | RegU64(o) | RegU128(o) | RegU256(o) | RegI8(o)
            | RegI16(o) | RegI32(o) | RegI64(o) | RegI128(o) | RegI256(o) => Some(*o),
            _ => None,
        }
    }

    /// True iff this operand is an immediate equal to zero.
    #[inline(always)]
    pub fn is_zero_imm(&self) -> bool {
        match self {
            IntOperand::ImmU8(v) => *v == 0,
            IntOperand::ImmU16(v) => *v == 0,
            IntOperand::ImmU32(v) => *v == 0,
            IntOperand::ImmU64(v) => *v == 0,
            IntOperand::ImmI8(v) => *v == 0,
            IntOperand::ImmI16(v) => *v == 0,
            IntOperand::ImmI32(v) => *v == 0,
            IntOperand::ImmI64(v) => *v == 0,
            IntOperand::ImmU128(b) => **b == 0,
            IntOperand::ImmI128(b) => **b == 0,
            IntOperand::ImmU256(b) => **b == U256::ZERO,
            IntOperand::ImmI256(b) => **b == I256::ZERO,
            _ => false,
        }
    }
}

impl fmt::Display for IntOperand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use IntOperand::*;
        match self {
            RegU8(o) => write!(f, "u8 [{}]", o.0),
            RegU16(o) => write!(f, "u16 [{}]", o.0),
            RegU32(o) => write!(f, "u32 [{}]", o.0),
            RegU64(o) => write!(f, "u64 [{}]", o.0),
            RegU128(o) => write!(f, "u128 [{}]", o.0),
            RegU256(o) => write!(f, "u256 [{}]", o.0),
            RegI8(o) => write!(f, "i8 [{}]", o.0),
            RegI16(o) => write!(f, "i16 [{}]", o.0),
            RegI32(o) => write!(f, "i32 [{}]", o.0),
            RegI64(o) => write!(f, "i64 [{}]", o.0),
            RegI128(o) => write!(f, "i128 [{}]", o.0),
            RegI256(o) => write!(f, "i256 [{}]", o.0),
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

/// `dst = lhs <kind> rhs`, where `kind` is determined by the wrapping
/// [`super::MicroOp`] variant ([`super::MicroOp::IntAdd`] …
/// [`super::MicroOp::IntBitXor`]). The operand type comes from `rhs`'s
/// [`IntOperand`] variant; `lhs` and `dst` are slots of the same width
/// (verifier-enforced). Signed-bitwise combinations are rejected by the
/// verifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntBinaryOp {
    pub dst: FrameOffset,
    pub lhs: FrameOffset,
    pub rhs: IntOperand,
}

/// The rhs of [`IntShiftOp`]. Move's shift amount is always a `u8`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShiftOperand {
    RegU8(FrameOffset),
    ImmU8(u8),
}

impl fmt::Display for ShiftOperand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ShiftOperand::RegU8(o) => write!(f, "[{}]", o.0),
            ShiftOperand::ImmU8(v) => write!(f, "#{}", v),
        }
    }
}

/// `dst = lhs <direction> rhs`, where direction is Shl
/// ([`super::MicroOp::IntShl`]) or Shr ([`super::MicroOp::IntShr`]).
/// Aborts if `rhs >= ty.bit_width()`.
///
/// TODO: revisit whether to fold this into [`IntBinaryOp`] (and let
/// [`IntOperand`] carry the always-`u8` shift amount). Keeping it separate
/// today since shifts don't share dispatch with the other binary ops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntShiftOp {
    pub ty: UnspecializedUnsignedIntTy,
    pub dst: FrameOffset,
    pub lhs: FrameOffset,
    pub rhs: ShiftOperand,
}

/// `dst = -src`. Aborts when `src == ty::MIN`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntNegateOp {
    pub ty: UnspecializedSignedIntTy,
    pub dst: FrameOffset,
    pub src: FrameOffset,
}
