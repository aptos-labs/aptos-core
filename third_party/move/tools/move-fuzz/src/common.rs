// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Types shared between the fuzzer driver and the subprocess it drives.
//!
//! # Why these exist instead of `SignatureToken` / `move_model::Type`
//!
//! The runbook and testnet paths do not call the VM in process. `simulator::run`
//! re-executes the fuzzer's own binary (the Aptos CLI, see `simulator::APTOS_BIN`)
//! as a subprocess:
//!
//! ```text
//! aptos move run --function-id 0x1::m::f --args u64:1 'address:[0x1, 0x2]'
//! ```
//!
//! and `testnet::create_bridge_script` emits a wrapper as Move *source*. So every
//! value has to cross a process boundary as text in the grammar the CLI's
//! `ArgWithType` parser accepts, and every type has to render as source syntax.
//! None of the general-purpose representations can do either:
//!
//! * `SignatureToken` is index-relative to the `CompiledModule` it came from, so it
//!   cannot be stored, compared or rendered on its own; it also admits everything
//!   the fuzzer cannot synthesize a value for.
//! * `move_model::ty::Type::from_signature_token` needs a `GlobalEnv` plus a struct
//!   resolver. The simulator only holds bare `CompiledModule`s read back from build
//!   artifacts, with no `GlobalEnv`.
//! * `move_core_types::transaction_argument::TransactionArgument` has no `String`,
//!   no `signer`, and only `U8Vector` rather than a general `vector<T>`.
//! * `move_core_types::value::MoveValue` erases a vector's element type, which is
//!   needed to prefix even an *empty* vector with its CLI type mark, and models
//!   `std::string::String` as an anonymous struct.
//! * `aptos_move_cli::FunctionArgType` is the closest fit, but it still is not a
//!   match (it has no `signer` and no vector variant -- vectors are a bracket in the
//!   value, not a mark) and, decisively, it lives in `aptos-move-cli`, which depends
//!   on this crate (`aptos-move/cli/src/fuzz.rs` imports `move_fuzz::cli`), so using
//!   it here would be a dependency cycle.
//!
//! `TxnArgType` is therefore a deliberate *whitelist*, not a type representation:
//! unsupported signatures are rejected once, at lookup time, instead of failing
//! later inside a subprocess.
//!
//! Conversions the binary format already implements are not re-implemented here:
//! drop-ability comes from `BinaryIndexedView::abilities`.


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
/// - variant *names*, because `Refty<TypeExpr>` and `Refty<TypeBase>` are
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

/// How a named address in a project's `Move.toml` is bound by the fuzzer.
///
/// This is a *declaration*, not a live account: it is built from CLI flags and
/// manifest named addresses before anything exists on chain, and the same value
/// drives both the in-process simulator and the `aptos` CLI profile used by the
/// testnet path. `aptos_language_e2e_tests::account::Account` models a funded
/// on-chain account and `LocalAccount` always owns a key, so neither can express
/// the `Ref` case (an address we hold no key for) or the `Resource` case (an
/// address derived from another account).
pub enum Account {
    /// A fixed address the fuzzer does not hold the key for.
    Ref(AccountAddress),
    /// An address the fuzzer generated and can sign for.
    Owned(Ed25519PrivateKey),
    /// A resource account derived from a base address and a seed.
    Resource(AccountAddress, String),
}

impl Account {
    /// The address an ed25519 key controls.
    ///
    /// The single definition of key-to-address derivation in this crate. Call sites
    /// that hold a bare key before an `Account` exists must use this instead of
    /// re-deriving, so the derivation stays in step with `AuthenticationKey`.
    pub fn address_of_key(key: &Ed25519PrivateKey) -> AccountAddress {
        AuthenticationKey::ed25519(&key.public_key()).account_address()
    }

    /// The address this binding resolves to.
    ///
    /// Derivation is delegated to the canonical helpers (`AuthenticationKey::ed25519`
    /// and `create_resource_address`) so it stays in step with the chain.
    pub fn address(&self) -> AccountAddress {
        match self {
            Self::Ref(addr) => *addr,
            Self::Owned(key) => Self::address_of_key(key),
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
    /// Project a `SignatureToken` onto the supported whitelist, or fail.
    ///
    /// Not a `TryFrom` impl because it needs the `BinaryIndexedView` the token was
    /// resolved against: `SignatureToken::Struct` carries only an index into that
    /// module's struct handle table, so `std::string::String` cannot be recognized
    /// without it. `StructTag::is_std_string` is not reused because building a
    /// `StructTag` from a `StructHandle` means allocating two `Identifier`s only to
    /// throw them away. The workspace has no `SignatureToken -> TypeTag` conversion
    /// to build on (`script_composer::helpers::import_type_tag` goes the other way).
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
            SignatureToken::I8
            | SignatureToken::I16
            | SignatureToken::I32
            | SignatureToken::I64
            | SignatureToken::I128
            | SignatureToken::I256 => {
                bail!("unexpected signed integer type in function signature")
            },
            // TODO: support generic structs/enums (planned)
            SignatureToken::StructInstantiation(..) => {
                bail!("unexpected generic struct in function signature")
            },
            SignatureToken::MutableReference(_) => {
                bail!("unexpected mutable reference in function signature")
            },
            SignatureToken::Function(..) => {
                bail!("unexpected function type in function signature")
            },
            SignatureToken::TypeParameter(_) => {
                bail!("unexpected type parameter in function signature")
            },
        };
        Ok(converted)
    }

    /// The type prefix used by `aptos move run --args <mark>:<value>`.
    ///
    /// For a vector this is the *element* mark, because the CLI spells a vector
    /// argument `u64:[1, 2]`, not `vector<u64>:[1, 2]`.
    ///
    /// The marks correspond to `aptos_move_cli::FunctionArgType`'s `FromStr`, which
    /// cannot be reused from here (dependency cycle -- see the module docs), with one
    /// deliberate exception: `Signer => "signer"` is *not* a mark that CLI accepts.
    /// It is unreachable in practice today because the only signer a runbook can
    /// name is the leading parameter, which `testnet::run_step` consumes as the
    /// transaction sender rather than passing through `--args`.
    /// TODO: reject non-leading `signer` parameters at lookup time so this cannot
    /// reach the subprocess at all.
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

    /// The Move *source* spelling of this type.
    ///
    /// Used to emit the generated bridge script in `testnet::create_bridge_script`,
    /// so it must be source syntax resolvable against that generated package's named
    /// addresses (`std::string::String`), not the canonical runtime rendering a
    /// `TypeTag` `Display` would produce (`0x1::string::String`).
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

/// A concrete argument value, in one-to-one correspondence with `TxnArgType`.
///
/// `move_core_types::value::MoveValue` is not used because it erases the element
/// type of a vector -- which is needed to prefix even an *empty* vector with its CLI
/// type mark -- and models `std::string::String` as an anonymous struct.
/// `TransactionArgument` is not used because it has no `String`, no `signer`, and
/// only `U8Vector` rather than a general `vector<T>`.
///
/// TODO: signed integer variants are omitted until `TxnArgType` grows them; nothing
/// can construct them before then. See the TODO on `TxnArgType`.
pub enum TxnArg {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    U256(U256),
    Address(AccountAddress),
    Signer(AccountAddress),
    String(String),
    /// The element type is carried explicitly so an empty vector still knows its
    /// CLI type mark.
    Vector(TxnArgType, Vec<TxnArg>),
}

impl TxnArg {
    /// The `<type>:<value>` string accepted by `aptos move run --args`.
    ///
    /// This is the single definition of the CLI argument encoding; callers shelling
    /// out to the CLI must not re-derive it.
    pub fn to_cli_arg(&self) -> String {
        format!("{}:{}", self.type_mark(), self.to_cli_string())
    }

    /// The CLI type prefix for this value, matching `TxnArgType::type_mark` (see the
    /// `signer` caveat there).
    fn type_mark(&self) -> &'static str {
        match self {
            Self::Bool(_) => "bool",
            Self::U8(_) => "u8",
            Self::U16(_) => "u16",
            Self::U32(_) => "u32",
            Self::U64(_) => "u64",
            Self::U128(_) => "u128",
            Self::U256(_) => "u256",
            Self::Address(_) => "address",
            Self::Signer(_) => "signer",
            Self::String(_) => "string",
            Self::Vector(sub, _) => sub.type_mark(),
        }
    }

    /// The value half of a CLI argument, without the type prefix. Recursive, so it
    /// must stay prefix-free for vector elements.
    pub fn to_cli_string(&self) -> String {
        match self {
            Self::Bool(b) => b.to_string(),
            Self::U8(n) => n.to_string(),
            Self::U16(n) => n.to_string(),
            Self::U32(n) => n.to_string(),
            Self::U64(n) => n.to_string(),
            Self::U128(n) => n.to_string(),
            Self::U256(n) => n.to_string(),
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
            SignatureToken::Bool
            | SignatureToken::U8
            | SignatureToken::U16
            | SignatureToken::U32
            | SignatureToken::U64
            | SignatureToken::U128
            | SignatureToken::U256
            | SignatureToken::I8
            | SignatureToken::I16
            | SignatureToken::I32
            | SignatureToken::I64
            | SignatureToken::I128
            | SignatureToken::I256
            | SignatureToken::Address
            | SignatureToken::Signer
            | SignatureToken::Vector(_)
            | SignatureToken::Function(..)
            | SignatureToken::Struct(_)
            | SignatureToken::StructInstantiation(..)
            | SignatureToken::TypeParameter(_) => Self::Base(TxnArgType::convert(binary, token)?),
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
