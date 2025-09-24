// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Representation of values (constants) in the assembler

use anyhow::anyhow;
use move_binary_format::file_format::SignatureToken;
use move_core_types::{
    account_address::AccountAddress,
    int256::{I256, U256},
    value::MoveValue,
};
use std::fmt;

/// An untyped numeric value, or a vector of such values.
#[derive(Debug)]
pub enum AsmValue {
    Number(/*sign_positive*/ bool, U256),
    Vector(Vec<AsmValue>),
}

impl AsmValue {
    pub fn unsigned(u256: U256) -> AsmValue {
        Self::Number(true, u256)
    }

    pub fn signed(i256: I256) -> AsmValue {
        if i256 < I256::ZERO {
            Self::Number(false, (-i256).try_into().expect("into succeeds"))
        } else {
            Self::Number(true, i256.try_into().expect("into succeeds"))
        }
    }
}

impl fmt::Display for AsmValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AsmValue::Number(s, v) => write!(f, "{}{}", if *s { "" } else { "-" }, v),
            AsmValue::Vector(vs) => {
                write!(f, "[")?;
                for (p, v) in vs.iter().enumerate() {
                    if p > 0 {
                        write!(f, ", ")?
                    }
                    write!(f, "{}", v)?
                }
                write!(f, "]")
            },
        }
    }
}

impl AsmValue {
    /// Converts untyped value into typed move value.
    pub fn to_move_value(&self, ty: &SignatureToken) -> anyhow::Result<MoveValue> {
        match ty {
            SignatureToken::Bool => {
                let num = self.check_unsigned_number(U256::ONE)?;
                Ok(MoveValue::Bool(num != U256::ZERO))
            },
            SignatureToken::U8 => Ok(MoveValue::U8(
                self.check_unsigned_number(U256::from(u8::MAX))?
                    .try_into()?,
            )),
            SignatureToken::U16 => Ok(MoveValue::U16(
                self.check_unsigned_number(U256::from(u16::MAX))?
                    .try_into()?,
            )),
            SignatureToken::U32 => Ok(MoveValue::U32(
                self.check_unsigned_number(U256::from(u32::MAX))?
                    .try_into()?,
            )),
            SignatureToken::U64 => Ok(MoveValue::U64(
                self.check_unsigned_number(U256::from(u64::MAX))?
                    .try_into()?,
            )),
            SignatureToken::U128 => Ok(MoveValue::U128(
                self.check_unsigned_number(U256::from(u128::MAX))?
                    .try_into()?,
            )),
            SignatureToken::U256 => Ok(MoveValue::U256(self.check_unsigned_number(U256::MAX)?)),
            SignatureToken::I8 => Ok(MoveValue::I8(
                self.check_signed_number(I256::from(i8::MIN), I256::from(i8::MAX))?
                    .try_into()?,
            )),
            SignatureToken::I16 => Ok(MoveValue::I16(
                self.check_signed_number(I256::from(i16::MIN), I256::from(i16::MAX))?
                    .try_into()?,
            )),
            SignatureToken::I32 => Ok(MoveValue::I32(
                self.check_signed_number(I256::from(i32::MIN), I256::from(i32::MAX))?
                    .try_into()?,
            )),
            SignatureToken::I64 => Ok(MoveValue::I64(
                self.check_signed_number(I256::from(i64::MIN), I256::from(i64::MAX))?
                    .try_into()?,
            )),
            SignatureToken::I128 => Ok(MoveValue::I128(
                self.check_signed_number(I256::from(i128::MIN), I256::from(i128::MAX))?
                    .try_into()?,
            )),
            SignatureToken::I256 => Ok(MoveValue::I256(
                self.check_signed_number(I256::MIN, I256::MAX)?,
            )),

            SignatureToken::Address => Ok(MoveValue::Address(u256_to_address(
                self.check_unsigned_number(U256::MAX)?,
            ))),
            SignatureToken::Vector(elem_type) => {
                if let AsmValue::Vector(vals) = self {
                    Ok(MoveValue::Vector(
                        vals.iter()
                            .map(|v| v.to_move_value(elem_type))
                            .collect::<anyhow::Result<Vec<_>>>()?,
                    ))
                } else {
                    Err(anyhow!("expected vector value"))
                }
            },
            SignatureToken::Signer
            | SignatureToken::Function(_, _, _)
            | SignatureToken::Struct(_)
            | SignatureToken::StructInstantiation(_, _)
            | SignatureToken::Reference(_)
            | SignatureToken::MutableReference(_)
            | SignatureToken::TypeParameter(_) => Err(anyhow!("invalid type for constant value")),
        }
    }

    pub fn check_unsigned_number(&self, max: U256) -> anyhow::Result<U256> {
        if let AsmValue::Number(true, n) = self {
            if *n <= max {
                Ok(*n)
            } else {
                Err(anyhow!("number {} out of range (max {})", n, max))
            }
        } else {
            Err(anyhow!("expected an unsigned number"))
        }
    }

    pub fn check_signed_number(&self, min: I256, max: I256) -> anyhow::Result<I256> {
        debug_assert!(min < I256::ZERO && max > I256::ZERO);
        let abs = |i: I256| -> anyhow::Result<U256> {
            if i == I256::MIN {
                // This value cannot be negated, so simulate conversion to U256
                let n: U256 = I256::MAX.try_into()?;
                Ok(U256::checked_add(n, U256::ONE).expect("add defined"))
            } else if i < I256::ZERO {
                Ok((-i).try_into()?)
            } else {
                Ok(i.try_into()?)
            }
        };
        if let AsmValue::Number(sign, n) = self {
            if *sign {
                if *n > max.try_into()? {
                    Err(anyhow!("number {} out of range (max {})", n, max))
                } else {
                    Ok((*n).try_into()?)
                }
            } else if *n > abs(min)? {
                Err(anyhow!("number -{} out of range (min {})", n, min))
            } else if *n
                == U256::checked_add(I256::MAX.try_into()?, 1u8.into()).expect("addition defined")
            {
                Ok(I256::MIN)
            } else {
                Ok(-(*n).try_into()?)
            }
        } else {
            Err(anyhow!("expected an signed number"))
        }
    }

    pub fn from_move_value(value: &MoveValue) -> anyhow::Result<AsmValue> {
        match value {
            MoveValue::Bool(v) => Ok(AsmValue::Number(
                true,
                U256::from(if *v { 1u8 } else { 0u8 }),
            )),
            MoveValue::U8(v) => Ok(AsmValue::Number(true, U256::from(*v))),
            MoveValue::U16(v) => Ok(AsmValue::Number(true, U256::from(*v))),
            MoveValue::U32(v) => Ok(AsmValue::Number(true, U256::from(*v))),
            MoveValue::U64(v) => Ok(AsmValue::Number(true, U256::from(*v))),
            MoveValue::U128(v) => Ok(AsmValue::Number(true, U256::from(*v))),
            MoveValue::U256(v) => Ok(AsmValue::Number(true, *v)),

            MoveValue::Address(v) => Ok(AsmValue::Number(true, address_to_u256(*v))),
            MoveValue::Vector(vs) => Ok(AsmValue::Vector(
                vs.iter()
                    .map(Self::from_move_value)
                    .collect::<anyhow::Result<Vec<_>>>()?,
            )),
            _ => Err(anyhow!("invalid constant value")),
        }
    }
}

pub(crate) fn u256_to_address(num: U256) -> AccountAddress {
    let mut bytes = num.to_le_bytes().to_vec();
    bytes.reverse();
    AccountAddress::from_bytes(bytes).expect("valid address value")
}

pub(crate) fn address_to_u256(addr: AccountAddress) -> U256 {
    let mut bytes = addr.into_bytes();
    bytes.reverse();
    U256::from_le_bytes(bytes)
}
