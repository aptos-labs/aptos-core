// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Result};
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey};
use aptos_types::{
    account_address::create_resource_address, transaction::authenticator::AuthenticationKey,
};
use move_binary_format::{binary_views::BinaryIndexedView, file_format::SignatureToken};
use move_core_types::{
    account_address::AccountAddress,
    int256::{I256, U256},
};
use move_model::model::AbilitySet;

/// Account (either referenced or owned)
pub enum Account {
    Ref(AccountAddress),
    Owned(Ed25519PrivateKey),
    Resource(AccountAddress, String),
}

impl Account {
    pub fn address(&self) -> AccountAddress {
        match self {
            Self::Ref(addr) => *addr,
            Self::Owned(key) => AuthenticationKey::ed25519(&key.public_key()).account_address(),
            Self::Resource(base, seed) => create_resource_address(*base, seed.as_bytes()),
        }
    }
}

/// Supported transaction argument types
#[derive(Clone)]
pub enum TxnArgType {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    Address,
    Signer,
    String,
    Vector(Box<TxnArgType>),
}

impl TxnArgType {
    pub fn convert(binary: BinaryIndexedView, token: &SignatureToken) -> Result<Self> {
        let converted = match token {
            SignatureToken::Bool => Self::Bool,
            SignatureToken::U8 => Self::U8,
            SignatureToken::U16 => Self::U16,
            SignatureToken::U32 => Self::U32,
            SignatureToken::U64 => Self::U64,
            SignatureToken::U128 => Self::U128,
            SignatureToken::U256 => Self::U256,
            SignatureToken::Address => Self::Address,
            SignatureToken::Signer => Self::Signer,
            SignatureToken::Struct(idx) => {
                let struct_handle = binary.struct_handle_at(*idx);
                let module_handle = binary.module_handle_at(struct_handle.module);
                if binary.identifier_at(struct_handle.name).as_str() == "String"
                    && binary.identifier_at(module_handle.name).as_str() == "string"
                    && binary.address_identifier_at(module_handle.address) == &AccountAddress::ONE
                {
                    Self::String
                } else {
                    bail!("unexpected struct in function signature");
                }
            },
            SignatureToken::Reference(sub) => {
                if matches!(sub.as_ref(), SignatureToken::Signer) {
                    Self::Signer
                } else {
                    bail!("unexpected reference in function signature");
                }
            },
            SignatureToken::Vector(sub) => Self::Vector(Self::convert(binary, sub)?.into()),
            _ => bail!("unexpected type in function signature"),
        };
        Ok(converted)
    }

    pub fn type_mark(&self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::U128 => "u128",
            Self::U256 => "u256",
            Self::Address => "address",
            Self::Signer => "signer",
            Self::String => "string",
            Self::Vector(sub) => sub.type_mark(),
        }
    }

    pub fn type_name(&self) -> String {
        match self {
            Self::Bool => "bool".to_string(),
            Self::U8 => "u8".to_string(),
            Self::U16 => "u16".to_string(),
            Self::U32 => "u32".to_string(),
            Self::U64 => "u64".to_string(),
            Self::U128 => "u128".to_string(),
            Self::U256 => "u256".to_string(),
            Self::Address => "address".to_string(),
            Self::Signer => "signer".to_string(),
            Self::String => "std::string::String".to_string(),
            Self::Vector(sub) => format!("vector<{}>", sub.type_name()),
        }
    }
}

/// Supported transaction argument
pub enum TxnArg {
    Bool(bool),
    U8(u8),
    I8(i8),
    U16(u16),
    I16(i16),
    U32(u32),
    I32(i32),
    U64(u64),
    I64(i64),
    U128(u128),
    I128(i128),
    U256(U256),
    I256(I256),
    Address(AccountAddress),
    Signer(AccountAddress),
    String(String),
    Vector(TxnArgType, Vec<TxnArg>),
}

impl TxnArg {
    pub fn to_cli_string(&self) -> String {
        match self {
            Self::Bool(b) => b.to_string(),
            Self::U8(n) => n.to_string(),
            Self::I8(n) => n.to_string(),
            Self::U16(n) => n.to_string(),
            Self::I16(n) => n.to_string(),
            Self::U32(n) => n.to_string(),
            Self::I32(n) => n.to_string(),
            Self::U64(n) => n.to_string(),
            Self::I64(n) => n.to_string(),
            Self::U128(n) => n.to_string(),
            Self::I128(n) => n.to_string(),
            Self::U256(n) => n.to_string(),
            Self::I256(n) => n.to_string(),
            Self::Address(a) => a.to_standard_string(),
            Self::Signer(a) => a.to_standard_string(),
            Self::String(s) => s.clone(),
            Self::Vector(_, sub) => {
                format!(
                    "[{}]",
                    sub.iter()
                        .map(|arg| arg.to_cli_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            },
        }
    }
}

/// Supported API (a.k.a., public function) argument types
#[derive(Clone)]
pub enum TxnArgTypeWithRef {
    Base(TxnArgType),
    RefImm(TxnArgType),
    RefMut(TxnArgType),
}

impl TxnArgTypeWithRef {
    pub fn convert(binary: BinaryIndexedView, token: &SignatureToken) -> Result<Self> {
        let converted = match token {
            SignatureToken::Reference(sub) => {
                Self::RefImm(TxnArgType::convert(binary, sub.as_ref())?)
            },
            SignatureToken::MutableReference(sub) => {
                Self::RefMut(TxnArgType::convert(binary, sub.as_ref())?)
            },
            _ => Self::Base(TxnArgType::convert(binary, token)?),
        };
        Ok(converted)
    }

    pub fn reduce(&self) -> TxnArgType {
        match self {
            Self::Base(ty) | Self::RefImm(ty) | Self::RefMut(ty) => ty.clone(),
        }
    }

    pub fn is_droppable(
        binary: BinaryIndexedView,
        generics: &[AbilitySet],
        token: &SignatureToken,
    ) -> bool {
        match token {
            SignatureToken::Bool
            | SignatureToken::U8
            | SignatureToken::I8
            | SignatureToken::U16
            | SignatureToken::I16
            | SignatureToken::U32
            | SignatureToken::I32
            | SignatureToken::U64
            | SignatureToken::I64
            | SignatureToken::U128
            | SignatureToken::I128
            | SignatureToken::U256
            | SignatureToken::I256
            | SignatureToken::Address
            | SignatureToken::Signer
            | SignatureToken::Reference(_)
            | SignatureToken::MutableReference(_)
            | SignatureToken::Function(..) => true,
            SignatureToken::Struct(idx) | SignatureToken::StructInstantiation(idx, _) => {
                let _ = idx;
                binary
                    .abilities(token, generics)
                    .map(|abilities| abilities.has_drop())
                    .unwrap_or(false)
            },
            SignatureToken::Vector(sub) => Self::is_droppable(binary, generics, sub.as_ref()),
            SignatureToken::TypeParameter(idx) => generics
                .get(*idx as usize)
                .expect("type parameter")
                .has_drop(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Account, TxnArg, TxnArgType, TxnArgTypeWithRef};
    use aptos_types::account_address::create_resource_address;
    use move_core_types::account_address::AccountAddress;

    #[test]
    fn test_resource_account_address_matches_move_derivation() {
        let base = AccountAddress::from_hex_literal("0x1").unwrap();
        let account = Account::Resource(base, "vault".to_string());
        assert_eq!(account.address(), create_resource_address(base, b"vault"));
    }

    #[test]
    fn test_txn_arg_type_names_and_marks_for_vectors() {
        let ty = TxnArgType::Vector(Box::new(TxnArgType::Vector(Box::new(TxnArgType::U64))));
        assert_eq!(ty.type_mark(), "u64");
        assert_eq!(ty.type_name(), "vector<vector<u64>>");
    }

    #[test]
    fn test_txn_arg_to_cli_string_formats_nested_vectors() {
        let arg = TxnArg::Vector(TxnArgType::Vector(Box::new(TxnArgType::U8)), vec![
            TxnArg::Vector(TxnArgType::U8, vec![TxnArg::U8(1), TxnArg::U8(2)]),
            TxnArg::Vector(TxnArgType::U8, vec![TxnArg::U8(3)]),
        ]);
        assert_eq!(arg.to_cli_string(), "[[1, 2], [3]]");
    }

    #[test]
    fn test_txn_arg_type_with_ref_reduce_preserves_base_type() {
        let ty = TxnArgTypeWithRef::RefMut(TxnArgType::Address);
        assert!(matches!(ty.reduce(), TxnArgType::Address));
    }
}
