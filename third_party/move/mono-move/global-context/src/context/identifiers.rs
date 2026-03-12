// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This submodule defines APIs to create and use identifiers (strings).
//!
//! # Safety model
//!
//! All arena string allocations produced here are wrapped in [`NameRef`],
//! which ties the validity of the underlying pointer to the  execution guard's
//! lifetime. The borrow checker therefore prevents any use of an allocation
//! after the guard is dropped, without requiring any runtime checks.

use crate::{alloc::GlobalArenaPtr, ExecutionGuard};
use dashmap::{Entry, Equivalent};
use move_core_types::identifier::IdentStr;
use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
};

/// A reference to interned name that can be used to identify a function or a
/// struct.
///
/// # Safety model
///
/// The reference lifetime is tied to the lifetime of the [`ExecutionGuard`].
/// It is guaranteed that the data it points to is kept alive as long as the
/// guard is alive.
pub struct NameRef<'a> {
    ptr: GlobalArenaPtr<str>,
    _guard: PhantomData<&'a ()>,
}

impl<'a> NameRef<'a> {
    /// Returns the inner string stored behind the reference.
    pub fn as_str(&self) -> &'a str {
        // SAFETY: The lifetime on this reference guarantees that the execution
        // guard is still alive, which guarantees that the arena allocation is
        // still valid and there were no deallocations.
        unsafe { self.ptr.as_ref_unchecked() }
    }

    /// Returns the raw address of the allocation of the pointer. For testing
    /// purposes only.
    pub fn raw_address_for_testing(&self) -> usize {
        self.ptr.as_raw_ptr().addr()
    }
}

impl<'a> Hash for NameRef<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ptr.hash(state)
    }
}

impl<'a> PartialEq for NameRef<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<'a> Eq for NameRef<'a> {}

impl<'a> Copy for NameRef<'a> {}

impl<'a> Clone for NameRef<'a> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a> ExecutionGuard<'a> {
    /// Interns Move identifier as a string and returns a reference to it. The
    /// reference is valid for the lifetime of the [`ExecutionGuard`].
    pub fn intern_identifier<'b>(&'b self, identifier: &IdentStr) -> NameRef<'b>
    where
        'a: 'b,
    {
        // TODO:
        //   Consider checking that the identifier size is within bounds. While
        //   CompiledModule / CompiledScript deserializer enforces 256 byte
        //   limit (in new config), when coming from deserialized TypeTag from
        //   transaction payload there is no bound. It is not a big problem,
        //   but just makes spam attacks easier to intern some dummy data in the
        //   pool. In general, for type tag interning we might want to enforce
        //   that the modules which are specified actually exist on-chain. In
        //   existing VM we already do that to get ability information, but not
        //   here (for now), so that we ensure that there is no spam that can
        //   get in. However, there still can be a problem with speculative
        //   module publishing: if we speculatively intern new names, but the
        //   publish actually fails, we end up with spam on-chain.
        //   Note: this DoS is only possible via `init_module`. If we remove it
        //   or ensure no speculative data even for names ever get on-chain, we
        //   limit interned set to the on-chain data, so for DoS one actually
        //   has to publish modules (expensive).
        let str = identifier.as_str();

        if let Some(name_ref) = self.get_interned_identifier(str) {
            return name_ref;
        };

        self.alloc_and_intern_identifier(str)
    }
}

//
// Only private APIs below.
// ------------------------

impl<'a> NameRef<'a> {
    /// Returns the raw global arena pointer to the allocated data.
    pub(super) fn as_global_arena_ptr(&self) -> GlobalArenaPtr<str> {
        self.ptr
    }
}

impl<'a> ExecutionGuard<'a> {
    /// Returns a reference to the identifier if it has been interned before.
    ///
    /// # Safety precondition
    ///
    /// During global arena reset, identifiers map must be cleared as well.
    fn get_interned_identifier<'b>(&'b self, str: &str) -> Option<NameRef<'b>>
    where
        'a: 'b,
    {
        self.ctx.identifiers.get(&LookupKey(str)).map(|entry| {
            // SAFETY: It is safe to cast its lifetime to the lifetime of the
            // execution guard. If the pointer existed before, it must still
            // be valid (during maintenance, if global arena is flushed, so is
            // the map).
            NameRef {
                ptr: *entry.value(),
                _guard: PhantomData,
            }
        })
    }

    /// On interner cache miss, allocates the string in the global arena and
    /// returns a pointer to:
    ///   - Allocated data if the string was not interned before.
    ///   - Existing canonical pointer if the interned string exists (this can
    ///     happen if there is a race condition).
    ///
    /// # Safety precondition
    ///
    /// During global arena reset, identifiers map must be cleared as well.
    fn alloc_and_intern_identifier<'b>(&'b self, str: &str) -> NameRef<'b>
    where
        'a: 'b,
    {
        // Allocate outside the lock to reduce contention. The leak is still
        // bounded to the number of concurrent workers, and therefore is
        // negligible in practice.
        let ptr = self.global_arena.alloc_str(str);

        // SAFETY: We have just allocated the pointer, hence dereferencing it to
        // compute the hash and equality is safe.
        let ptr = match self.ctx.identifiers.entry(IdentifierInternerKey(ptr)) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => *entry.insert(ptr),
        };

        // SAFETY: The allocated pointer is trivially valid until the next
        // maintenance so it is safe to cast its lifetime to the lifetime of
        // the execution guard. If the pointer existed before, it must still
        // be valid (if global arena was flushed, so must have been the map).
        NameRef {
            ptr,
            _guard: PhantomData,
        }
    }
}

/// Wraps allocated string pointer to implement structural hash and equality.
///
/// # Safety
///
/// APIs must enforce the pointer points to valid data and can be dereferenced.
pub(super) struct IdentifierInternerKey(GlobalArenaPtr<str>);

// Wrapper to avoid orphan rule.
#[derive(Hash, PartialEq, Eq)]
struct LookupKey<'a>(&'a str);

impl Hash for IdentifierInternerKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: must be enforced by the caller.
        let str = unsafe { self.0.as_ref_unchecked() };
        str.hash(state);
    }
}

impl PartialEq for IdentifierInternerKey {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: must be enforced by the caller.
        unsafe { self.0.as_ref_unchecked() == other.0.as_ref_unchecked() }
    }
}

impl Eq for IdentifierInternerKey {}

impl Equivalent<IdentifierInternerKey> for LookupKey<'_> {
    fn equivalent(&self, key: &IdentifierInternerKey) -> bool {
        // SAFETY: must be enforced by the caller.
        let key_ref = unsafe { key.0.as_ref_unchecked() };
        self.0 == key_ref
    }
}
