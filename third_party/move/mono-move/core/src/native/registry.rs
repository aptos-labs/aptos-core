// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native function registry: a lookup table the VM uses to resolve a
//! native function from either its fully-qualified name (during
//! specialization) or its [`NativeIdx`] (at runtime dispatch).

use super::context::NativeContext;
use crate::{
    interner::{InternedIdentifier, InternedModuleId},
    native::NativeStatus,
    types::InternedTypeList,
    VMResult,
};
use shared_dsa::UnorderedMap;
use thiserror::Error;

/// Index into the natives registry's table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct NativeIdx(pub u32);

/// Key under which a native is registered in a [`NativeRegistry`].
///
/// A native is either a template that serves every instantiation, or a body
/// specialized to one concrete instantiation. The two variants let
/// type-agnostic natives register once while type-sensitive natives register a
/// separate body per concrete type. Resolution tries the monomorphic key first
/// and falls back to the polymorphic one (see [`NativeRegistry::resolve`]).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeName {
    /// One implementation for all instantiations.
    Polymorphic {
        module: InternedModuleId,
        function: InternedIdentifier,
    },
    /// Implementation specialized to one concrete instantiation, matched only
    /// when the call's type arguments equal `ty_args`.
    Monomorphic {
        module: InternedModuleId,
        function: InternedIdentifier,
        ty_args: InternedTypeList,
    },
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
pub type NativeFunction<F> = Box<
    dyn for<'a> Fn(&<F as NativeContextFamily>::Of<'a>) -> VMResult<NativeStatus> + Send + Sync,
>;

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

#[derive(Debug, Error)]
pub enum NativeRegistryError {
    #[error("native already registered under this name")]
    DuplicateName,
}

/// Lookup table of native functions, generic over a [`NativeContextFamily`] `F`.
pub struct NativeRegistry<F: NativeContextFamily> {
    entries: Vec<NativeFunction<F>>,
    by_name: UnorderedMap<NativeName, NativeIdx>,
}

impl<F: NativeContextFamily> NativeRegistry<F> {
    /// Creates an empty native registry.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            by_name: UnorderedMap::default(),
        }
    }

    /// Number of registered natives.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Register a native under the given fully-qualified name.
    pub fn register(
        &mut self,
        name: NativeName,
        func: NativeFunction<F>,
    ) -> Result<NativeIdx, NativeRegistryError> {
        if self.by_name.contains_key(&name) {
            return Err(NativeRegistryError::DuplicateName);
        }
        let idx = NativeIdx(self.entries.len() as u32);
        self.entries.push(func);
        self.by_name.insert(name, idx);
        Ok(idx)
    }

    /// Register many natives at once. Returns indices in input order;
    /// short-circuits on the first error.
    pub fn register_all<I>(&mut self, natives: I) -> Result<Vec<NativeIdx>, NativeRegistryError>
    where
        I: IntoIterator<Item = (NativeName, NativeFunction<F>)>,
    {
        natives
            .into_iter()
            .map(|(name, func)| self.register(name, func))
            .collect()
    }

    /// Look up the function pointer for a [`NativeIdx`].
    #[inline]
    pub fn lookup_by_idx(&self, idx: NativeIdx) -> Option<&NativeFunction<F>> {
        self.entries.get(idx.0 as usize)
    }

    /// Look up the [`NativeIdx`] of a fully-qualified name.
    pub fn lookup_by_name(&self, name: &NativeName) -> Option<NativeIdx> {
        self.by_name.get(name).copied()
    }
}

impl<F: NativeContextFamily> Default for NativeRegistry<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: NativeContextFamily> NativeResolver for NativeRegistry<F> {
    /// Resolves the monomorphic registration for `ty_args` if one exists,
    /// otherwise falls back to a polymorphic (template) registration.
    fn resolve(
        &self,
        module: InternedModuleId,
        function: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> Option<NativeIdx> {
        self.lookup_by_name(&NativeName::Monomorphic {
            module,
            function,
            ty_args,
        })
        .or_else(|| self.lookup_by_name(&NativeName::Polymorphic { module, function }))
    }
}
