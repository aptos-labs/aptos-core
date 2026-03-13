// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This submodule defines APIs to create and use identifiers (strings) and
//! executable IDs.
//!
//! # Safety model
//!
//! All arena allocations produced here are wrapped in [`Ref`], which ties the
//! validity of the underlying pointer to the [`ExecutionGuard`]'s lifetime.
//! The borrow checker therefore prevents any use of an allocation after the
//! guard is dropped, without requiring any runtime checks.

use super::Ref;
use crate::{alloc::GlobalArenaPtr, ExecutionGuard};
use move_core_types::account_address::AccountAddress;
use std::marker::PhantomData;

impl<'a> Ref<'a, str> {
    /// Returns the inner string stored behind the reference.
    pub fn as_str(&self) -> &'a str {
        // SAFETY: The lifetime on this reference guarantees that the execution
        // guard is still alive, which guarantees that the arena allocation is
        // still valid and there were no deallocations.
        unsafe { self.ptr.as_ref_unchecked() }
    }
}

/// Identifies an executable (module or script) by its address and name.
///
/// # Invariant
///
/// Only [`ExecutionGuard::alloc_executable_id`] can create [`ExecutableId`].
/// No external code can construct or inspect fields directly; access goes
/// through [`Ref`].
pub struct ExecutableId {
    pub(super) address: AccountAddress,
    pub(super) name: GlobalArenaPtr<str>,
}

impl<'a> Ref<'a, ExecutableId> {
    /// Returns the account address of this executable.
    pub fn address(&self) -> &'a AccountAddress {
        &self.as_ref().address
    }

    /// Returns the name of this executable.
    pub fn name(&self) -> &'a str {
        // SAFETY: The lifetime on this reference guarantees that the execution
        // guard is still alive. Because name was arena-allocated before this
        // ID was created (enforced by global arena allocator APIs), the name
        // allocation is also valid for the same lifetime.
        unsafe { self.as_ref().name.as_ref_unchecked() }
    }
}

//
// Only private APIs below.
// ------------------------

impl<'a> ExecutionGuard<'a> {
    /// Allocates a string in the arena, returning a reference to it with the
    /// lifetime tied to the lifetime of [`ExecutionGuard`]. This is **the
    /// only** way to create a [`Ref`] to the allocated string.
    pub(super) fn alloc_str<'b>(&'b self, s: &str) -> Ref<'b, str>
    where
        'a: 'b,
    {
        Ref {
            ptr: self.global_arena.alloc_str(s),
            _guard: PhantomData,
        }
    }

    /// Allocates [`ExecutableId`] in the arena, returning a reference to it
    /// with the lifetime tied to the lifetime of [`ExecutionGuard`]. This is
    /// **the only** way to create a [`Ref`] to [`ExecutableId`].
    pub(super) fn alloc_executable_id<'b>(
        &'b self,
        address: AccountAddress,
        name: Ref<'b, str>,
    ) -> Ref<'b, ExecutableId>
    where
        'a: 'b,
    {
        // SAFETY: Extracting the raw pointer here is safe because the returned
        // ID pointer is immediately re-wrapped under the same guards lifetime.
        let Ref { ptr: name, _guard } = name;
        Ref {
            ptr: self.global_arena.alloc(ExecutableId { address, name }),
            _guard,
        }
    }
}

impl<'a> Ref<'a, ExecutableId> {
    fn as_ref(&self) -> &'a ExecutableId {
        // SAFETY: The lifetime on this reference guarantees that the execution
        // guard is still alive, which guarantees that the arena allocation is
        // still valid and there were no deallocations.
        unsafe { self.ptr.as_ref_unchecked() }
    }
}
