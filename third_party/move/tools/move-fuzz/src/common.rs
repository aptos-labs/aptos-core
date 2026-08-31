// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{bail, Result};
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey};
use aptos_types::{
    account_address::create_resource_address, transaction::authenticator::AuthenticationKey,
};
use move_binary_format::{binary_views::BinaryIndexedView, file_format::SignatureToken};
use move_core_types::{
    ability::AbilitySet,
    account_address::AccountAddress,
    int256::{I256, U256},
};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Move's three reference modes applied to a base type `T`.
///
/// Move has no references to references, so a reference mode is a single,
/// non-nestable wrapper around a base type. This crate used to carry four
/// hand-written copies of these same three variants: `TxnArgTypeWithRef` here,
/// `prep::typing::TypeRef`, `prep::typing::TypeItem` and
/// `prep::graph::DatatypeItem`. All four are now aliases of this one enum.
///
/// Two things about this type are load-bearing and must not change:
/// - variant *order*, because `Ord` is derived and the aliases are used as
///   `BTreeMap` / `BTreeSet` keys and in sorted output;
/// - variant *names*, because `Refty<TypeTag>` and `Refty<TypeBase>` are
///   serialized into the on-disk entrypoint cache (`PersistedEntrypointCache`).
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Serialize, Deserialize)]
pub enum Refty<T> {
    /// `T`
    Base(T),
    /// `&T`
    ImmRef(T),
    /// `&mut T`
    MutRef(T),
}

impl<T> Refty<T> {
    /// The base type, with the reference mode dropped.
    pub fn base(&self) -> &T {
        match self {
            Self::Base(t) | Self::ImmRef(t) | Self::MutRef(t) => t,
        }
    }

    /// Rewrite the base type, keeping the reference mode.
    pub fn map<U>(&self, f: impl FnOnce(&T) -> U) -> Refty<U> {
        match self {
            Self::Base(t) => Refty::Base(f(t)),
            Self::ImmRef(t) => Refty::ImmRef(f(t)),
            Self::MutRef(t) => Refty::MutRef(f(t)),
        }
    }
}

impl<T: Clone> Refty<T> {
    /// An owned copy of the base type, with the reference mode dropped.
    pub fn reduce(&self) -> T {
        self.base().clone()
    }
}

impl<T: Display> Display for Refty<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Base(t) => write!(f, "{t}"),
            Self::ImmRef(t) => write!(f, "&{t}"),
            Self::MutRef(t) => write!(f, "&mut {t}"),
        }
    }
}

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

/// Supported transaction argument types.
///
/// This is the vocabulary of the *external* driver (`simulator` / `testnet`),
/// which runs the `aptos` CLI as a subprocess: `type_mark` is literally the
/// CLI's `--args` marker (`u64:`, `string:`, ...) and `type_name` is the Move
/// source spelling used in a generated bridge script. It is therefore a
/// deliberately small, closed set, and `convert` *failing* on anything else is
/// the check that decides whether a function is runnable through the CLI at all.
///
/// Why not `move_core_types::language_storage::TypeTag`: that type has no notion
/// of "expressible as a CLI argument", so adopting it would turn the check above
/// into a second, separate validation pass over an already-converted type.
///
/// Why not `crate::prep::canvas::BasicInput`: `BasicInput` is total by
/// construction (it is produced by the script synthesizer, never parsed from
/// user input) and can name objects and script generics, neither of which the
/// CLI path can carry.
///
/// TODO: signed integer types (i8, i16, i32, i64, i128, i256) are not yet
/// supported here.
/// TODO: public structs/enums (other than `std::string::String`) are not yet
/// supported; support is planned.
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
                    // TODO: support public structs/enums (planned)
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
            // TODO: support signed integer types (i8, i16, i32, i64, i128, i256)
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

/// Supported API (a.k.a., public function) argument types: a [`TxnArgType`]
/// under one of Move's three reference modes; see [`Refty`].
pub type TxnArgTypeWithRef = Refty<TxnArgType>;

impl Refty<TxnArgType> {
    pub fn convert(binary: BinaryIndexedView, token: &SignatureToken) -> Result<Self> {
        let converted = match token {
            SignatureToken::Reference(sub) => {
                Self::ImmRef(TxnArgType::convert(binary, sub.as_ref())?)
            },
            SignatureToken::MutableReference(sub) => {
                Self::MutRef(TxnArgType::convert(binary, sub.as_ref())?)
            },
            _ => Self::Base(TxnArgType::convert(binary, token)?),
        };
        Ok(converted)
    }

    /// Whether a value of type `token` can be silently discarded, i.e., whether
    /// the type has the `drop` ability. `generics` supplies the ability
    /// constraints of the enclosing function's type parameters.
    ///
    /// This delegates to `BinaryIndexedView::abilities`
    /// (`third_party/move/move-binary-format/src/binary_views.rs`), the same
    /// helper the bytecode verifier uses, so the fuzzer cannot drift from the
    /// VM's notion of abilities.
    ///
    /// Note in particular that function values are *not* unconditionally
    /// droppable. The abilities of a function type are exactly the ones written
    /// on the type itself (`Function(_, _, abilities) => Ok(*abilities)`), so a
    /// bare `|u64|u64` has no abilities at all; the runtime applies the same
    /// rule in `Type::abilities`
    /// (`move-vm/types/src/loaded_data/runtime_types.rs`). Captured values are
    /// already folded into that ability set when the closure is built:
    /// `clos_pack` in `move-bytecode-verifier/src/type_safety.rs` (and
    /// `verify_pack_closure` in `move-vm/runtime/src/runtime_type_checks.rs`)
    /// starts from `AbilitySet::PRIVATE_FUNCTIONS`/`PUBLIC_FUNCTIONS` and
    /// intersects the abilities of every captured argument, so a closure over a
    /// non-droppable capture already carries a function type without `drop`.
    /// There is nothing extra to compute here.
    ///
    /// A resolver error (only reachable for a malformed module) is reported as
    /// "not droppable" so callers skip the function instead of emitting a bridge
    /// script that fails to compile.
    pub fn is_droppable(
        binary: BinaryIndexedView,
        generics: &[AbilitySet],
        token: &SignatureToken,
    ) -> bool {
        binary
            .abilities(token, generics)
            .map(|abilities| abilities.has_drop())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::{Account, TxnArg, TxnArgType, TxnArgTypeWithRef};
    use aptos_types::account_address::create_resource_address;
    use move_binary_format::{
        binary_views::BinaryIndexedView,
        file_format::{empty_module, SignatureToken},
    };
    use move_core_types::{
        ability::{Ability, AbilitySet},
        account_address::AccountAddress,
    };

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
        let ty = TxnArgTypeWithRef::MutRef(TxnArgType::Address);
        assert!(matches!(ty.reduce(), TxnArgType::Address));
    }

    /// A function value is droppable exactly when its type declares `drop`.
    /// See `BinaryIndexedView::abilities`: `Function(_, _, abilities) =>
    /// Ok(*abilities)`.
    #[test]
    fn test_is_droppable_for_function_values_follows_declared_abilities() {
        fn fun_ty(abilities: AbilitySet) -> SignatureToken {
            SignatureToken::Function(
                vec![SignatureToken::U64],
                vec![SignatureToken::U64],
                abilities,
            )
        }

        let module = empty_module();
        let view = BinaryIndexedView::Module(&module);

        // `|u64|u64` carries no abilities at all, so it must not be dropped.
        // This is also the shape produced by `PackClosure` when a capture is
        // not droppable, since the verifier intersects the captured argument
        // abilities into the resulting function type.
        assert!(!TxnArgTypeWithRef::is_droppable(
            view,
            &[],
            &fun_ty(AbilitySet::EMPTY)
        ));
        // `|u64|u64 has copy` still cannot be dropped.
        assert!(!TxnArgTypeWithRef::is_droppable(
            view,
            &[],
            &fun_ty(AbilitySet::EMPTY.add(Ability::Copy))
        ));
        // `|u64|u64 has drop` can.
        assert!(TxnArgTypeWithRef::is_droppable(
            view,
            &[],
            &fun_ty(AbilitySet::EMPTY.add(Ability::Drop))
        ));
        // ... and non-droppability propagates through `vector<_>`.
        assert!(!TxnArgTypeWithRef::is_droppable(
            view,
            &[],
            &SignatureToken::Vector(Box::new(fun_ty(AbilitySet::EMPTY)))
        ));
        assert!(TxnArgTypeWithRef::is_droppable(
            view,
            &[],
            &SignatureToken::Vector(Box::new(fun_ty(AbilitySet::EMPTY.add(Ability::Drop))))
        ));
    }

    #[test]
    fn test_is_droppable_for_primitives_references_and_type_params() {
        let module = empty_module();
        let view = BinaryIndexedView::Module(&module);

        for token in [
            SignatureToken::Bool,
            SignatureToken::U64,
            SignatureToken::U256,
            SignatureToken::Address,
            SignatureToken::Signer,
            SignatureToken::Reference(Box::new(SignatureToken::U64)),
            SignatureToken::MutableReference(Box::new(SignatureToken::U64)),
            SignatureToken::Vector(Box::new(SignatureToken::U8)),
        ] {
            assert!(TxnArgTypeWithRef::is_droppable(view, &[], &token));
        }

        // Type parameters take the abilities of their declared constraints, and
        // `vector<T>` is droppable exactly when `T` is.
        let constraints = [AbilitySet::EMPTY, AbilitySet::EMPTY.add(Ability::Drop)];
        assert!(!TxnArgTypeWithRef::is_droppable(
            view,
            &constraints,
            &SignatureToken::TypeParameter(0)
        ));
        assert!(TxnArgTypeWithRef::is_droppable(
            view,
            &constraints,
            &SignatureToken::TypeParameter(1)
        ));
        assert!(!TxnArgTypeWithRef::is_droppable(
            view,
            &constraints,
            &SignatureToken::Vector(Box::new(SignatureToken::TypeParameter(0)))
        ));
    }
}
