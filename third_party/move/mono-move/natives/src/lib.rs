// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native function implementations shipped with MonoMove. Each native is a
//! plain function generic over any [`NativeContext`] impl.
//!
//! [`NativeContext`]: mono_move_core::native::NativeContext

// Re-exported so the native list macros can name these via `$crate::...`
// without callers having to add `mono-move-core` to their imports.
use mono_move_core::native::NativeContextFamily;
pub use mono_move_core::native::{Dispatch, NativeFunction, NativeName};
use move_core_types::{account_address::AccountAddress, identifier::Identifier};

pub mod aggregator_v2;
pub mod aptos_hash;
pub mod bcs;
pub mod bls12381;
pub mod bulletproofs;
pub mod cmp;
pub mod consensus_config;
pub mod ed25519;
pub mod event;
pub mod from_bytes;
pub mod function_info;
pub mod hash;
pub mod init;
pub mod mem;
pub mod multi_ed25519;
pub mod object;
pub mod ristretto255_point;
pub mod ristretto255_scalar;
pub mod secp256k1;
pub mod signer;
pub mod state_storage;
pub mod string;
pub mod table;
#[cfg(feature = "testing")]
pub mod test_natives;
pub mod transaction_context;
pub mod type_info;
#[cfg(feature = "testing")]
pub mod unit_test;
pub mod vector;

pub use aggregator_v2::make_all_aggregator_v2_natives;
pub use aptos_hash::make_all_aptos_hash_natives;
pub use bcs::make_all_bcs_natives;
pub use bls12381::make_all_bls12381_natives;
#[cfg(feature = "testing")]
pub use bls12381::make_all_bls12381_test_natives;
pub use bulletproofs::make_all_bulletproofs_natives;
#[cfg(feature = "testing")]
pub use bulletproofs::make_all_bulletproofs_test_natives;
pub use cmp::make_all_cmp_natives;
pub use consensus_config::make_all_consensus_config_natives;
pub use ed25519::make_all_ed25519_natives;
#[cfg(feature = "testing")]
pub use ed25519::make_all_ed25519_test_natives;
pub use event::{make_all_event_natives, EventEntry, EventKind, EventStore};
pub use from_bytes::make_all_from_bytes_natives;
pub use function_info::make_all_function_info_natives;
pub use hash::make_all_hash_natives;
pub use init::make_all_init_natives;
pub use mem::make_all_mem_natives;
pub use multi_ed25519::make_all_multi_ed25519_natives;
#[cfg(feature = "testing")]
pub use multi_ed25519::make_all_multi_ed25519_test_natives;
pub use object::{make_all_object_natives, ObjectContextExtension};
pub use ristretto255_point::{make_all_ristretto255_point_natives, RistrettoPointStore};
pub use ristretto255_scalar::make_all_ristretto255_scalar_natives;
#[cfg(feature = "testing")]
pub use ristretto255_scalar::make_all_ristretto255_scalar_test_natives;
pub use secp256k1::make_all_secp256k1_natives;
pub use signer::make_all_signer_natives;
pub use state_storage::{make_all_state_storage_natives, StorageUsageAtEpochBoundary};
pub use string::make_all_string_natives;
pub use table::make_all_table_natives;
#[cfg(feature = "testing")]
pub use test_natives::{
    make_all_test_natives, native_split_bytes, native_u64_add, native_u64_identity,
};
pub use transaction_context::{make_all_transaction_context_natives, TransactionContextExtension};
pub use type_info::make_all_type_info_natives;
#[cfg(feature = "testing")]
pub use unit_test::make_all_unit_test_natives;
pub use vector::make_all_vector_natives;

/// One native registration: its name and its implementation.
pub type NativeEntry<F> = (NativeName, NativeFunction<F>);

/// All natives shipped with the production MonoMove VM. Additional native
/// modules are concatenated here as they are implemented.
pub fn make_all_production_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    let mut natives = make_all_signer_natives::<F>();
    natives.extend(make_all_mem_natives::<F>());
    natives.extend(make_all_type_info_natives::<F>());
    natives.extend(make_all_function_info_natives::<F>());
    natives.extend(make_all_aggregator_v2_natives::<F>());
    natives.extend(make_all_transaction_context_natives::<F>());
    natives.extend(make_all_object_natives::<F>());
    natives.extend(make_all_state_storage_natives::<F>());
    natives.extend(make_all_event_natives::<F>());
    natives.extend(make_all_hash_natives::<F>());
    natives.extend(make_all_aptos_hash_natives::<F>());
    natives.extend(make_all_init_natives::<F>());
    natives.extend(make_all_string_natives::<F>());
    natives.extend(make_all_bcs_natives::<F>());
    natives.extend(make_all_cmp_natives::<F>());
    natives.extend(make_all_consensus_config_natives::<F>());
    natives.extend(make_all_from_bytes_natives::<F>());
    natives.extend(make_all_table_natives::<F>());
    natives.extend(make_all_secp256k1_natives::<F>());
    natives.extend(make_all_ed25519_natives::<F>());
    natives.extend(make_all_multi_ed25519_natives::<F>());
    natives.extend(make_all_bls12381_natives::<F>());
    natives.extend(make_all_ristretto255_scalar_natives::<F>());
    natives.extend(make_all_ristretto255_point_natives::<F>());
    natives.extend(make_all_bulletproofs_natives::<F>());
    natives.extend(make_all_vector_natives::<F>());
    natives
}

/// Parses a fully-qualified function name (e.g. "0x1::natives::u64_add")
/// into its component parts. Panics on malformed input.
//
// TODO(cleanup): replace with a proper parser. See if one already exists in
// move-core-types.
// TODO(cleanup): make this a `const fn` so malformed names are caught at
// compile time rather than when the native list is built at startup.
pub(crate) fn parse_qualified_native_name(
    qname: &'static str,
) -> (AccountAddress, &'static str, &'static str) {
    let mut parts = qname.split("::");
    let addr_part = parts
        .next()
        .unwrap_or_else(|| panic!("malformed native name `{}`: missing address", qname));
    let module = parts
        .next()
        .unwrap_or_else(|| panic!("malformed native name `{}`: missing module", qname));
    if !Identifier::is_valid(module) {
        panic!(
            "malformed native name `{}`:` module name is not a valid identifier",
            qname
        );
    }
    let function = parts
        .next()
        .unwrap_or_else(|| panic!("malformed native name `{}`: missing function", qname));
    if !Identifier::is_valid(function) {
        panic!(
            "malformed native name `{}`: function name is not a valid identifier",
            qname
        );
    }
    assert!(
        parts.next().is_none(),
        "malformed native name `{}`: too many `::`-separated parts",
        qname,
    );
    let addr = AccountAddress::from_hex_literal(addr_part)
        .unwrap_or_else(|e| panic!("malformed native name `{}`: invalid address: {}", qname, e));
    (addr, module, function)
}

/// Builds one [`NativeEntry`].
macro_rules! native_entry {
    ($qname:expr, $dispatch:expr, $func:expr) => {{
        // The `wrapper` is a generic function whose lifetime is late-bound, so
        // `wrapper::<F>` coerces to the higher-ranked:
        // ```
        //   for<'a> fn(&F::Of<'a>) -> ..
        // ```
        // that the native table stores.
        fn wrapper<F: ::mono_move_core::native::NativeContextFamily>(
            ctx: &<F as ::mono_move_core::native::NativeContextFamily>::Of<'_>,
        ) -> ::mono_move_core::VMResult<::mono_move_core::native::NativeStatus> {
            $func(ctx)
        }

        let (address, module, function) = $crate::parse_qualified_native_name($qname);
        let name = $crate::NativeName {
            address,
            module,
            function,
            dispatch: $dispatch,
        };
        (name, wrapper::<F> as $crate::NativeFunction<F>)
    }};
}
pub(crate) use native_entry;

/// Builds a list of monomorphic natives: each body is registered for one
/// concrete instantiation, and the specializer resolves it from the call's type
/// arguments. Each entry is either `(name, fn)` — empty type arguments (a
/// non-generic native) — or `(name, &[ty, ...], fn)` to specialize on concrete
/// type arguments. Each element of the type-argument slice must be a `'static`
/// primitive type constant (e.g. `U64_TY`), never an arena-interned type.
//
// TODO(testing): debug-assert this in the registry builder instead of relying
// on the doc invariant.
///
/// Example:
/// ```ignore
/// let natives = monomorphic_natives![
///     ("0x1::aggregator_v2::try_add", &[U64_TY], native_try_add::<_, u64>),
///     ("0x1::aggregator_v2::try_add", &[U128_TY], native_try_add::<_, u128>),
/// ];
/// ```
macro_rules! monomorphic_natives {
    [ $( $entry:tt ),* $(,)? ] => {
        ::std::vec![ $( $crate::monomorphic_natives!(@entry $entry) ),* ]
    };
    (@entry ( $qname:expr , $ty_args:expr , $func:expr )) => {
        $crate::native_entry!($qname, $crate::Dispatch::Monomorphic($ty_args), $func)
    };
    (@entry ( $qname:expr , $func:expr )) => {
        $crate::native_entry!($qname, $crate::Dispatch::Monomorphic(&[]), $func)
    };
}
pub(crate) use monomorphic_natives;

/// Builds a list of polymorphic natives: each body works for any instantiation
/// and the specializer resolves it from the call's module and function name
/// alone. Each entry is `(name, fn)`.
///
/// Example:
/// ```ignore
/// let natives = polymorphic_natives![
///     ("0x1::mem::swap", native_swap),
/// ];
/// ```
macro_rules! polymorphic_natives {
    [ $( ( $qname:expr , $func:expr ) ),* $(,)? ] => {
        ::std::vec![ $(
            $crate::native_entry!($qname, $crate::Dispatch::Polymorphic, $func)
        ),* ]
    };
}
pub(crate) use polymorphic_natives;
