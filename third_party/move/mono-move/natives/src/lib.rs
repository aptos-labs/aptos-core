// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native function implementations shipped with MonoMove. Each native is a
//! plain function generic over any [`NativeContext`] impl.
//!
//! [`NativeContext`]: mono_move_core::native::NativeContext

// Re-exported so the `natives!` macro can name it via `$crate::NativeFunction`
// without callers having to add `mono-move-core` to their imports.
pub use mono_move_core::native::NativeFunction;
use mono_move_core::{native::NativeContextFamily, types::InternedType};
use move_core_types::{account_address::AccountAddress, identifier::Identifier};

pub mod aggregator_v2;
pub mod function_info;
pub mod mem;
pub mod signer;
pub mod test_natives;
pub mod type_info;
pub mod vector;

pub use aggregator_v2::make_all_aggregator_v2_natives;
pub use function_info::make_all_function_info_natives;
pub use mem::make_all_mem_natives;
pub use signer::make_all_signer_natives;
pub use test_natives::{make_all_test_natives, native_u64_add, native_u64_identity};
pub use type_info::make_all_type_info_natives;
pub use vector::make_all_vector_natives;

/// How a native is dispatched against a call's type arguments. A native that
/// works for any instantiation registers as [`Dispatch::Polymorphic`]; a native
/// specialized to one concrete instantiation registers as
/// [`Dispatch::Monomorphic`] carrying that instantiation's type arguments. The
/// consumer interns these into a `NativeName` registry key.
//
// TODO: this duplicates `NativeName`'s shape. We keep it separate because
// `NativeName` holds arena-interned ids that require an `ExecutionGuard`, which
// is unavailable when these tables are built statically. Revisit if interning
// becomes available earlier.
pub enum Dispatch {
    Polymorphic,
    Monomorphic(&'static [InternedType]),
}

/// One native registration: its qualified name parts, its dispatch kind, and
/// the boxed implementation.
pub type NativeEntry<F> = (
    AccountAddress,
    Identifier,
    Identifier,
    Dispatch,
    NativeFunction<F>,
);

/// All natives shipped with the production MonoMove VM. Additional native
/// modules are concatenated here as they are implemented.
pub fn make_all_production_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    let mut natives = make_all_signer_natives::<F>();
    natives.extend(make_all_mem_natives::<F>());
    natives.extend(make_all_type_info_natives::<F>());
    natives.extend(make_all_function_info_natives::<F>());
    natives.extend(make_all_aggregator_v2_natives::<F>());
    natives.extend(make_all_vector_natives::<F>());
    natives
}

/// Parses a fully-qualified function name (e.g. "0x1::natives::u64_add")
/// into its component parts. Panics on malformed input.
//
// TODO: replace with a proper parser. See if one already exists in
// move-core-types.
pub(crate) fn parse_qualified_native_name(qname: &str) -> (AccountAddress, Identifier, Identifier) {
    let mut parts = qname.split("::");
    let addr_part = parts
        .next()
        .unwrap_or_else(|| panic!("malformed native name `{}`: missing address", qname));
    let module_part = parts
        .next()
        .unwrap_or_else(|| panic!("malformed native name `{}`: missing module", qname));
    let function_part = parts
        .next()
        .unwrap_or_else(|| panic!("malformed native name `{}`: missing function", qname));
    assert!(
        parts.next().is_none(),
        "malformed native name `{}`: too many `::`-separated parts",
        qname,
    );
    let addr = AccountAddress::from_hex_literal(addr_part)
        .unwrap_or_else(|e| panic!("malformed native name `{}`: invalid address: {}", qname, e));
    let module = Identifier::new(module_part)
        .unwrap_or_else(|e| panic!("malformed native name `{}`: invalid module: {}", qname, e));
    let function = Identifier::new(function_part)
        .unwrap_or_else(|e| panic!("malformed native name `{}`: invalid function: {}", qname, e));
    (addr, module, function)
}

/// Builds one [`NativeEntry`]: parses the qualified name, boxes the function,
/// and pairs it with the given [`Dispatch`]. Shared by the list macros below.
macro_rules! native_entry {
    ($qname:expr, $dispatch:expr, $func:expr) => {{
        let (addr, module, function) = $crate::parse_qualified_native_name($qname);
        let func: $crate::NativeFunction<_> = ::std::boxed::Box::new(|ctx| $func(ctx));
        (addr, module, function, $dispatch, func)
    }};
}
pub(crate) use native_entry;

/// Builds a list of monomorphic natives: each body is registered for one
/// concrete instantiation, and the specializer resolves it from the call's type
/// arguments. Each entry is either `(name, fn)` — empty type arguments (a
/// non-generic native) — or `(name, &[ty, ...], fn)` to specialize on the given
/// static type arguments. The caller supplies the concrete types (e.g. the
/// turbofished function and `&[U64_TY]`).
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
