// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Type definitions for the global execution context.

use crate::{alloc::GlobalArenaPtr, context::ListRef, ExecutableId, ExecutionGuard, Ref};
use move_core_types::ability::AbilitySet;

/// Runtime type representation.
///
/// # Invariant
///
/// While [`Type`] is a public enum (for convenience), it is only ever exposed
/// in public APIs through [`Ref`] or [`ListRef`]. These references are scoped
/// to the lifetime of the [`ExecutionGuard`] and are safe to use while holding
/// the execution guard. Only allocation methods in this submodule can create
/// reference to types and pointing to the global arena.
pub enum Type {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    I8,
    I16,
    I32,
    I64,
    I128,
    I256,
    Address,
    Signer,
    Vector(GlobalArenaPtr<Type>),
    Ref(GlobalArenaPtr<Type>),
    RefMut(GlobalArenaPtr<Type>),
    Struct {
        // TODO:
        //   Currently, we have 3 pointers here which bloat the size of the
        //   enum (in addition to function value below). If this is ever a
        //   problem, we can consider moving some pieces into their own
        //   allocations, for example, defining a pointer for struct definition
        //   (executable ID plus name). For now, this design suffices to get
        //   things running.
        /// Executable ID (address and name).
        executable_id: GlobalArenaPtr<ExecutableId>,
        /// Struct name.
        name: GlobalArenaPtr<str>,
        /// Type arguments.
        type_args: GlobalArenaPtr<[GlobalArenaPtr<Type>]>,
    },
    Function {
        /// Argument types.
        args: GlobalArenaPtr<[GlobalArenaPtr<Type>]>,
        /// Return types.
        results: GlobalArenaPtr<[GlobalArenaPtr<Type>]>,
        /// Abilities of the function.
        abilities: AbilitySet,
    },
    /// A type parameter. Substituted at monomorphization time and the type is
    /// **re-canonicalized**.
    TypeParam(u16),
}

//
// Only private APIs below.
// ------------------------

impl<'a> ExecutionGuard<'a> {
    /// Allocates a vector of the specified type in the arena, returning a
    /// reference to it with the same lifetime as [`ExecutionGuard`]'s. This is
    /// **the only** way to create a vector [`Ref`].
    pub(super) fn alloc_vector_type<'b>(&'b self, elem_type: Ref<'b, Type>) -> Ref<'b, Type>
    where
        'a: 'b,
    {
        // SAFETY: Extracting the raw pointer here is safe because the returned
        // ID pointer is immediately re-wrapped under the same guards lifetime.
        let Ref {
            ptr: elem_type,
            _guard,
        } = elem_type;
        Ref {
            ptr: self.global_arena.alloc(Type::Vector(elem_type)),
            _guard,
        }
    }

    /// Allocates a reference type in the arena, returning a reference to the
    /// allocated data with the same lifetime as [`ExecutionGuard`]'s. This is
    /// **the only** way to create a such a [`Ref`].
    pub(super) fn alloc_ref_type<'b>(&'b self, elem_type: Ref<'b, Type>) -> Ref<'b, Type>
    where
        'a: 'b,
    {
        // SAFETY: Extracting the raw pointer here is safe because the returned
        // pointer is immediately re-wrapped under the same guards lifetime.
        let Ref {
            ptr: elem_type,
            _guard,
        } = elem_type;
        Ref {
            ptr: self.global_arena.alloc(Type::Ref(elem_type)),
            _guard,
        }
    }

    /// Allocates a mutable reference type in the arena, returning a reference
    /// to the allocated data with the same lifetime as [`ExecutionGuard`]'s.
    /// This is **the only** way to create a such a [`Ref`].
    pub(super) fn alloc_ref_mut_type<'b>(&'b self, elem_type: Ref<'b, Type>) -> Ref<'b, Type>
    where
        'a: 'b,
    {
        // SAFETY: Extracting the raw pointer here is safe because the returned
        // pointer is immediately re-wrapped under the same guards lifetime.
        let Ref {
            ptr: elem_type,
            _guard,
        } = elem_type;
        Ref {
            ptr: self.global_arena.alloc(Type::RefMut(elem_type)),
            _guard,
        }
    }

    pub(super) fn alloc_struct_type<'b>(
        &'b self,
        executable_id: Ref<'b, ExecutableId>,
        name: Ref<'b, str>,
        type_args: ListRef<'b, Type>,
    ) -> Ref<'b, Type>
    where
        'a: 'b,
    {
        // SAFETY: Extracting raw pointers here is safe because the returned
        // pointers are immediately re-wrapped under the same guards lifetime.
        let Ref {
            ptr: executable_id,
            _guard,
        } = executable_id;
        let Ref { ptr: name, .. } = name;
        let ListRef { ptr: type_args, .. } = type_args;

        Ref {
            ptr: self.global_arena.alloc(Type::Struct {
                executable_id,
                name,
                type_args,
            }),
            _guard,
        }
    }

    pub(super) fn alloc_function_type<'b>(
        &'b self,
        args: ListRef<'b, Type>,
        results: ListRef<'b, Type>,
        abilities: AbilitySet,
    ) -> Ref<'b, Type>
    where
        'a: 'b,
    {
        // SAFETY: Extracting raw pointers here is safe because the returned
        // pointers are immediately re-wrapped under the same guards lifetime.
        let ListRef { ptr: args, _guard } = args;
        let ListRef { ptr: results, .. } = results;

        Ref {
            ptr: self.global_arena.alloc(Type::Function {
                args,
                results,
                abilities,
            }),
            _guard,
        }
    }
}

/// Static allocation for boolean type.
pub(crate) static BOOL: Type = Type::Bool;

/// Static allocation for u8 type.
pub(crate) static U8: Type = Type::U8;

/// Static allocation for u16 type.
pub(crate) static U16: Type = Type::U16;

/// Static allocation for u32 type.
pub(crate) static U32: Type = Type::U32;

/// Static allocation for u64 type.
pub(crate) static U64: Type = Type::U64;

/// Static allocation for u128 type.
pub(crate) static U128: Type = Type::U128;

/// Static allocation for u256 type.
pub(crate) static U256: Type = Type::U256;

/// Static allocation for i8 type.
pub(crate) static I8: Type = Type::I8;

/// Static allocation for i16 type.
pub(crate) static I16: Type = Type::I16;

/// Static allocation for i32 type.
pub(crate) static I32: Type = Type::I32;

/// Static allocation for i64 type.
pub(crate) static I64: Type = Type::I64;

/// Static allocation for i128 type.
pub(crate) static I128: Type = Type::I128;

/// Static allocation for i256 type.
pub(crate) static I256: Type = Type::I256;

/// Static allocation for address type.
pub(crate) static ADDRESS: Type = Type::Address;

/// Static allocation for signer type.
pub(crate) static SIGNER: Type = Type::Signer;
