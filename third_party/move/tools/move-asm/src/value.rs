// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Representation of values (constants) in the assembler

use anyhow::anyhow;
use move_binary_format::file_format::SignatureToken;
use move_core_types::{account_address::AccountAddress, u256::U256, value::MoveValue};
use std::fmt;

/// An untyped numeric value, or a vector of such values.
#[derive(Debug)]
pub enum AsmValue {
    Number(U256),
    Vector(Vec<AsmValue>),
}

impl fmt::Display for AsmValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AsmValue::Number(v) => write!(f, "{}", v),
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
                let num = self.check_number(U256::from(1u8))?;
                Ok(MoveValue::Bool(num != U256::zero()))
            },
            SignatureToken::U8 => Ok(MoveValue::U8(
                self.check_number(U256::from(u8::MAX))?.unchecked_as_u8(),
            )),
            SignatureToken::U16 => Ok(MoveValue::U16(
                self.check_number(U256::from(u16::MAX))?.unchecked_as_u16(),
            )),
            SignatureToken::U32 => Ok(MoveValue::U32(
                self.check_number(U256::from(u32::MAX))?.unchecked_as_u32(),
            )),
            SignatureToken::U64 => Ok(MoveValue::U64(
                self.check_number(U256::from(u64::MAX))?.unchecked_as_u64(),
            )),
            SignatureToken::U128 => Ok(MoveValue::U128(
                self.check_number(U256::from(u128::MAX))?
                    .unchecked_as_u128(),
            )),
            SignatureToken::U256 => Ok(MoveValue::U256(self.check_number(U256::max_value())?)),
            SignatureToken::Address => Ok(MoveValue::Address(u256_to_address(
                self.check_number(U256::max_value())?,
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

    pub fn check_number(&self, max: U256) -> anyhow::Result<U256> {
        if let AsmValue::Number(n) = self {
            if *n <= max {
                Ok(*n)
            } else {
                Err(anyhow!("number {} out of range (max {})", n, max))
            }
        } else {
            Err(anyhow!("expected a number but found a vector"))
        }
    }

    pub fn from_move_value(value: &MoveValue) -> anyhow::Result<AsmValue> {
        match value {
            MoveValue::Bool(v) => Ok(AsmValue::Number(U256::from(if *v { 1u8 } else { 0u8 }))),
            MoveValue::U8(v) => Ok(AsmValue::Number(U256::from(*v))),
            MoveValue::U16(v) => Ok(AsmValue::Number(U256::from(*v))),
            MoveValue::U32(v) => Ok(AsmValue::Number(U256::from(*v))),
            MoveValue::U64(v) => Ok(AsmValue::Number(U256::from(*v))),
            MoveValue::U128(v) => Ok(AsmValue::Number(U256::from(*v))),
            MoveValue::U256(v) => Ok(AsmValue::Number(*v)),
            MoveValue::Address(v) => Ok(AsmValue::Number(address_to_u256(*v))),
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
    U256::from_le_bytes(&bytes)
}
