// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native function registry primitives shared across the VM.
//!
//! - [`NativeFunction`], the raw fn-pointer type a runtime's registry stores,
//!   and [`NativeIdx`], the position in that table.
//! - [`NativeResolver`], the trait the loader and specializer use to resolve a
//!   native call by its name and type arguments to its [`NativeIdx`].

use super::context::NativeContext;
use crate::{
    interner::{InternedIdentifier, InternedModuleId},
    native::NativeStatus,
    types::{InternedType, InternedTypeList},
    VMResult,
};
use move_core_types::account_address::AccountAddress;

/// Index into the natives registry's table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct NativeIdx(pub u32);

/// Describes how a native is dispatched.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dispatch {
    /// A native that serves every possible instantiation.
    Polymorphic,
    /// A native that is specialized to a single concrete instantiation or does
    /// not carry type arguments at all.
    Monomorphic(&'static [InternedType]),
}

/// Describes a natives function: its module address and name, function name
/// and how it should be dispatched.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeDescriptor {
    pub address: AccountAddress,
    pub module: &'static str,
    pub function: &'static str,
    pub dispatch: Dispatch,
}

/// Describes a family of [`NativeContext`] types indexed by a lifetime.
///
/// See [`NativeFunction`] for why this is needed.
pub trait NativeContextFamily {
    /// The native context type for a per-call borrow of lifetime `'a`.
    type Of<'a>: NativeContext + 'a;
}

/// A native function pointer stored in a [`NativeRegistry`].
///
/// Note that it needs to be parametric over not just a single context type, but
/// an entire family of context types ([`NativeContextFamily`]), parameterized over a
/// lifetime (of the borrows of VM components like the gas meter, determined at
/// individual native call sites).
///
/// Without this, we cannot store native functions in a registry, which would
/// otherwise mandate a fixed lifetime.
pub type NativeFunction<F> =
    for<'a> fn(&<F as NativeContextFamily>::Of<'a>) -> VMResult<NativeStatus>;

/// Resolves a native call to its [`NativeIdx`] from the callee's qualified name
/// and the call's concrete type arguments.
pub trait NativeResolver {
    fn resolve(
        &self,
        module: InternedModuleId,
        function: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> Option<NativeIdx>;
}

/// A [`NativeResolver`] that resolves nothing -- useful for tests and simulations that
/// don't have any natives.
pub struct NoNatives;

impl NativeResolver for NoNatives {
    fn resolve(
        &self,
        _module: InternedModuleId,
        _function: InternedIdentifier,
        _ty_args: InternedTypeList,
    ) -> Option<NativeIdx> {
        None
    }
}
