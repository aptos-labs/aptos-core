// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This submodule defines APIs to create and use types or lists of types.
//!
//! # Safety model
//!
//! All arena type allocations produced here are wrapped in [`TypeRef`] or
//! [`TypeListRef`], which tie the validity of the underlying pointer to the
//! execution guard's lifetime. The borrow checker therefore prevents any use
//! of an allocation after the guard is dropped, without requiring any runtime
//! checks.
//!
//! **Primitive types** (bool, integers, address, signer) are backed by static
//! allocation, so they are never invalidated by an arena reset.
//!
//! **Empty type lists** are also backed by a static zero-length array,
//! avoiding any arena allocation for the very common non-generic type case.
//!
//! **Composite types** (vectors, structs, functions, references) are allocated
//! in the global arena. The arena is only reset during the maintenance phase,
//! which cannot overlap with any live [`ExecutionGuard`], so all pointers
//! returned during execution remain valid for the guard's lifetime.

use crate::{
    alloc::GlobalArenaPtr, context::executable_ids::ExecutableId, ExecutableIdRef, ExecutionGuard,
    NameRef,
};
use dashmap::{Entry, Equivalent};
use move_core_types::{
    ability::AbilitySet,
    language_storage::{FunctionParamOrReturnTag, TypeTag},
};
use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
};

/// Structural size metrics stored inline in every arena-allocated type.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TypeTreeSize {
    /// Node count in a full tree traversal (shared subnodes counted multiple times).
    pub count: u32,
    /// Maximum root-to-leaf depth.
    pub depth: u32,
}

/// Error returned when interning a composite type fails due to configured
/// depth or size bounds.
///
/// # Note
///
/// The full error handling strategy for MonoMove has not yet been settled.
/// This type is intentionally kept minimal — new variants will be added as
/// the strategy is decided.
#[derive(Debug)]
pub enum TypeError {
    TypeTooDeep { depth: u32, max: u32 },
    TypeTooLarge { count: u32, max: u32 },
}

/// Runtime representation of a composite interned type, stored in the arena.
///
/// Wraps the semantic [`Type`] variant and pre-computes structural metadata
/// at interning time so that downstream consumers (e.g., monomorphization) can
/// query size/depth in O(1) without re-traversal.
///
/// # Future extensions
///
/// When a module cache is available, this struct can be extended to cache
/// per-type information that currently requires a module lookup:
///
/// - `abilities: AbilitySet` — requires loading the struct definition from its
///   module to determine which abilities the type satisfies. Currently there is
///   no module cache in `global-context`, so this field is deferred.
///   TODO: add `abilities: AbilitySet` once a module cache is wired in.
/// - Type layout / size in bytes — similar module-loading requirement.
pub(super) struct RuntimeTypeInfo {
    pub(super) ty: Type,
    pub(super) size: TypeTreeSize,
}

/// A reference to interned type.
///
/// # Safety model
///
/// The reference lifetime is tied to the lifetime of the [`ExecutionGuard`].
/// It is guaranteed that the data it points to is kept alive as long as the
/// guard is alive.
#[repr(transparent)]
pub struct TypeRef<'a> {
    ptr: GlobalArenaPtr<RuntimeTypeInfo>,
    _guard: PhantomData<&'a ()>,
}

impl<'a> TypeRef<'a> {
    /// Returns the raw address of the allocation of the pointer. For testing
    /// purposes only.
    pub fn raw_address_for_testing(&self) -> usize {
        self.ptr.as_raw_ptr().addr()
    }

    /// Returns the structural size metrics for this type.
    pub fn size(&self) -> TypeTreeSize {
        // SAFETY: guard lifetime ensures the arena allocation is still valid.
        unsafe { self.ptr.as_ref_unchecked() }.size
    }
}

impl<'a> Hash for TypeRef<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ptr.hash(state)
    }
}

impl<'a> PartialEq for TypeRef<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<'a> Eq for TypeRef<'a> {}

impl<'a> Copy for TypeRef<'a> {}

impl<'a> Clone for TypeRef<'a> {
    fn clone(&self) -> Self {
        *self
    }
}

/// A reference to interned type list.
///
/// # Safety model
///
/// The reference lifetime is tied to the lifetime of the [`ExecutionGuard`].
/// It is guaranteed that the data it points to is kept alive as long as the
/// guard is alive.
#[repr(transparent)]
pub struct TypeListRef<'a> {
    ptr: GlobalArenaPtr<[GlobalArenaPtr<RuntimeTypeInfo>]>,
    _guard: PhantomData<&'a ()>,
}

impl<'a> TypeListRef<'a> {
    /// Returns the raw address of the allocation of the pointer. For testing
    /// purposes only.
    pub fn raw_address_for_testing(&self) -> usize {
        self.ptr.as_raw_ptr().addr()
    }
}

impl<'a> Hash for TypeListRef<'a> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ptr.hash(state)
    }
}

impl<'a> PartialEq for TypeListRef<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl<'a> Eq for TypeListRef<'a> {}

impl<'a> Copy for TypeListRef<'a> {}

impl<'a> Clone for TypeListRef<'a> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a> ExecutionGuard<'a> {
    /// Interns a [`TypeTag`] and returns a reference to it. The reference is
    /// valid for the lifetime of the [`ExecutionGuard`].
    pub fn intern_type_tag<'b>(&'b self, ty_tag: &TypeTag) -> Result<TypeRef<'b>, TypeError>
    where
        'a: 'b,
    {
        match ty_tag {
            TypeTag::Bool => return Ok(static_type_ref(&BOOL)),
            TypeTag::U8 => return Ok(static_type_ref(&U8)),
            TypeTag::U16 => return Ok(static_type_ref(&U16)),
            TypeTag::U32 => return Ok(static_type_ref(&U32)),
            TypeTag::U64 => return Ok(static_type_ref(&U64)),
            TypeTag::U128 => return Ok(static_type_ref(&U128)),
            TypeTag::U256 => return Ok(static_type_ref(&U256)),
            TypeTag::I8 => return Ok(static_type_ref(&I8)),
            TypeTag::I16 => return Ok(static_type_ref(&I16)),
            TypeTag::I32 => return Ok(static_type_ref(&I32)),
            TypeTag::I64 => return Ok(static_type_ref(&I64)),
            TypeTag::I128 => return Ok(static_type_ref(&I128)),
            TypeTag::I256 => return Ok(static_type_ref(&I256)),
            TypeTag::Address => return Ok(static_type_ref(&ADDRESS)),
            TypeTag::Signer => return Ok(static_type_ref(&SIGNER)),
            TypeTag::Vector(..) | TypeTag::Struct(..) | TypeTag::Function(..) => {
                // Composite types are allocated in global arena, handled below.
            },
        }

        if let Some(ty_ref) = self.get_interned_type(&LookupKey(ty_tag)) {
            return Ok(ty_ref);
        }

        match ty_tag {
            TypeTag::Vector(elem_tag) => {
                let elem_ty_ref = self.intern_type_tag(elem_tag.as_ref())?;
                self.allocate_and_intern_nested_type(elem_ty_ref, Type::Vector)
            },
            TypeTag::Struct(struct_tag) => {
                let executable_id_ref =
                    self.intern_address_name(&struct_tag.address, &struct_tag.module);
                let name_ref = self.intern_identifier(&struct_tag.name);
                let ty_args_list_ref = self.intern_type_tags(&struct_tag.type_args)?;
                self.allocate_and_intern_struct_type(executable_id_ref, name_ref, ty_args_list_ref)
            },
            TypeTag::Function(function_tag) => {
                let args =
                    self.intern_function_param_or_return_type_tags(function_tag.args.as_ref())?;
                let results =
                    self.intern_function_param_or_return_type_tags(function_tag.results.as_ref())?;
                self.allocate_and_intern_function_type(args, results, function_tag.abilities)
            },

            TypeTag::Bool
            | TypeTag::U8
            | TypeTag::U64
            | TypeTag::U128
            | TypeTag::Address
            | TypeTag::Signer
            | TypeTag::U16
            | TypeTag::U32
            | TypeTag::U256
            | TypeTag::I8
            | TypeTag::I16
            | TypeTag::I32
            | TypeTag::I64
            | TypeTag::I128
            | TypeTag::I256 => unreachable!("Already handled above"),
        }
    }

    /// Interns a list of [`TypeTag`]s and returns a reference to it. The
    /// reference is valid for the lifetime of the [`ExecutionGuard`].
    pub fn intern_type_tags<'b>(&'b self, ty_tags: &[TypeTag]) -> Result<TypeListRef<'b>, TypeError>
    where
        'a: 'b,
    {
        if ty_tags.is_empty() {
            return Ok(static_type_list_ref(&EMPTY_TYPE_LIST));
        }

        if let Some(ty_ref) = self.get_interned_type_list(&LookupKey(ty_tags)) {
            return Ok(ty_ref);
        }

        let types = ty_tags
            .iter()
            .map(|tag| {
                // SAFETY: By construction, element type pointer is valid. Note
                // that **all** arenas in the global arena pool are reset
                // together so this is safe.
                Ok(self.intern_type_tag(tag)?.as_global_arena_ptr())
            })
            .collect::<Result<Vec<_>, TypeError>>()?;

        // Allocate outside the lock to reduce contention. The leak is still
        // bounded to the number of concurrent workers, and therefore is
        // negligible in practice.
        // TODO:
        //   1. For lists, this might not be negligible if the list is large?
        //   2. Consider using alloc_slice_fill_iter.
        let ptr = self.global_arena.alloc_slice_copy(&types);

        // SAFETY: We have just allocated the type list pointer, hence
        // dereferencing it to compute the hash and equality is safe (for
        // transitive pointers as well).
        let ptr = match self.ctx.type_lists.entry(TypeListInternerKey(ptr)) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => *entry.insert(ptr),
        };

        // SAFETY: The allocated pointer is trivially valid until the next
        // maintenance so it is safe to cast its lifetime to the lifetime of
        // the execution guard. If the pointer existed before, it must still
        // be valid (if global arena was flushed, so must have been the map).
        Ok(TypeListRef {
            ptr,
            _guard: PhantomData,
        })
    }
}

//
// Only private APIs below.
// ------------------------

impl<'a> TypeRef<'a> {
    /// Returns the raw global arena pointer to the allocated data.
    pub(super) fn as_global_arena_ptr(&self) -> GlobalArenaPtr<RuntimeTypeInfo> {
        self.ptr
    }
}

impl<'a> TypeListRef<'a> {
    /// Returns the raw global arena pointer to the allocated data.
    pub(super) fn as_global_arena_ptr(&self) -> GlobalArenaPtr<[GlobalArenaPtr<RuntimeTypeInfo>]> {
        self.ptr
    }
}

/// Runtime type representation. For internal usage only.
///
/// # Invariant
///
/// Only [`ExecutionGuard`] methods can create pointers to [`Type`]s. No
/// external code or file can construct or inspect types directly; access goes
/// through [`TypeRef`] or [`TypeListRef`].
// TODO:
//   Currently, enum has huge size. If this is ever a problem, consider moving
//   some pieces into their own allocations.
pub(super) enum Type {
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
    Vector(GlobalArenaPtr<RuntimeTypeInfo>),
    Ref(GlobalArenaPtr<RuntimeTypeInfo>),
    RefMut(GlobalArenaPtr<RuntimeTypeInfo>),
    Struct {
        /// Executable ID (address and name).
        executable_id: GlobalArenaPtr<ExecutableId>,
        /// Struct name.
        name: GlobalArenaPtr<str>,
        /// Type arguments.
        ty_args: GlobalArenaPtr<[GlobalArenaPtr<RuntimeTypeInfo>]>,
    },
    Function {
        /// Argument types.
        args: GlobalArenaPtr<[GlobalArenaPtr<RuntimeTypeInfo>]>,
        /// Return types.
        results: GlobalArenaPtr<[GlobalArenaPtr<RuntimeTypeInfo>]>,
        /// Abilities of the function.
        abilities: AbilitySet,
    },
    /// A type parameter. Substituted at monomorphization time and the type is
    /// **re-canonicalized**.
    TypeParam(u16),
}

/// Creates a [`RuntimeTypeInfo`] for a primitive type (count=1, depth=1).
const fn prim(ty: Type) -> RuntimeTypeInfo {
    RuntimeTypeInfo {
        ty,
        size: TypeTreeSize { count: 1, depth: 1 },
    }
}

static BOOL: RuntimeTypeInfo = prim(Type::Bool);
static U8: RuntimeTypeInfo = prim(Type::U8);
static U16: RuntimeTypeInfo = prim(Type::U16);
static U32: RuntimeTypeInfo = prim(Type::U32);
static U64: RuntimeTypeInfo = prim(Type::U64);
static U128: RuntimeTypeInfo = prim(Type::U128);
static U256: RuntimeTypeInfo = prim(Type::U256);
static I8: RuntimeTypeInfo = prim(Type::I8);
static I16: RuntimeTypeInfo = prim(Type::I16);
static I32: RuntimeTypeInfo = prim(Type::I32);
static I64: RuntimeTypeInfo = prim(Type::I64);
static I128: RuntimeTypeInfo = prim(Type::I128);
static I256: RuntimeTypeInfo = prim(Type::I256);
static ADDRESS: RuntimeTypeInfo = prim(Type::Address);
static SIGNER: RuntimeTypeInfo = prim(Type::Signer);
static EMPTY_TYPE_LIST: [GlobalArenaPtr<RuntimeTypeInfo>; 0] = [];

/// Canonical discriminants for cross-format hashing. This ensures that **ALL**
/// keys hash to the same value.
mod type_discriminant {
    pub(super) const BOOL: u8 = 0;
    pub(super) const U8: u8 = 1;
    pub(super) const U16: u8 = 2;
    pub(super) const U32: u8 = 3;
    pub(super) const U64: u8 = 4;
    pub(super) const U128: u8 = 5;
    pub(super) const U256: u8 = 6;
    pub(super) const I8: u8 = 7;
    pub(super) const I16: u8 = 8;
    pub(super) const I32: u8 = 9;
    pub(super) const I64: u8 = 10;
    pub(super) const I128: u8 = 11;
    pub(super) const I256: u8 = 12;
    pub(super) const ADDRESS: u8 = 13;
    pub(super) const SIGNER: u8 = 14;
    pub(super) const VECTOR: u8 = 15;
    pub(super) const STRUCT: u8 = 16;
    pub(super) const REFERENCE: u8 = 17;
    pub(super) const REFERENCE_MUT: u8 = 18;
    pub(super) const FUNCTION: u8 = 19;
    pub(super) const TYPE_PARAM: u8 = 20;
}

/// Wraps allocated type pointers to implement structural hash and
/// equality.
///
/// # Safety precondition
///
/// APIs must enforce the pointer points to valid data and can be dereferenced.
pub(super) struct TypeInternerKey(GlobalArenaPtr<RuntimeTypeInfo>);

/// Wraps allocated type list pointers to implement structural hash and
/// equality.
///
/// # Safety precondition
///
/// APIs must enforce the pointer points to valid data and can be dereferenced.
pub(super) struct TypeListInternerKey(GlobalArenaPtr<[GlobalArenaPtr<RuntimeTypeInfo>]>);

/// Wrapper around [`TypeTag`] or [`SignatureToken`] to implement same hashing
/// as [`TypeInternerKey`] and equivalence.
struct LookupKey<'a, T: ?Sized>(&'a T);

impl<'a> ExecutionGuard<'a> {
    /// Interns a [`FunctionParamOrReturnTag`] and returns a reference to it.
    /// The reference is valid for the lifetime of the [`ExecutionGuard`].
    pub fn intern_function_param_or_return_type_tag<'b>(
        &'b self,
        ty_tag: &FunctionParamOrReturnTag,
    ) -> Result<TypeRef<'b>, TypeError>
    where
        'a: 'b,
    {
        if let FunctionParamOrReturnTag::Value(ty_tag) = ty_tag {
            return self.intern_type_tag(ty_tag);
        }

        if let Some(ty_ref) = self.get_interned_type(&LookupKey(ty_tag)) {
            return Ok(ty_ref);
        }

        match ty_tag {
            FunctionParamOrReturnTag::Reference(inner_tag) => {
                let inner_ty_ref = self.intern_type_tag(inner_tag)?;
                self.allocate_and_intern_nested_type(inner_ty_ref, Type::Ref)
            },
            FunctionParamOrReturnTag::MutableReference(inner_tag) => {
                let inner_ty_ref = self.intern_type_tag(inner_tag)?;
                self.allocate_and_intern_nested_type(inner_ty_ref, Type::RefMut)
            },
            FunctionParamOrReturnTag::Value(..) => unreachable!("Already handled"),
        }
    }

    /// Interns a list of [`FunctionParamOrReturnTag`]s and returns a reference
    /// to it. The reference is valid for the lifetime of the [`ExecutionGuard`].
    pub fn intern_function_param_or_return_type_tags<'b>(
        &'b self,
        ty_tags: &[FunctionParamOrReturnTag],
    ) -> Result<TypeListRef<'b>, TypeError>
    where
        'a: 'b,
    {
        if ty_tags.is_empty() {
            return Ok(static_type_list_ref(&EMPTY_TYPE_LIST));
        }

        if let Some(ty_ref) = self.get_interned_type_list(&LookupKey(ty_tags)) {
            return Ok(ty_ref);
        }

        let types = ty_tags
            .iter()
            .map(|tag| {
                // SAFETY: By construction, element type pointer is valid. Note
                // that **all** arenas in the global arena pool are reset
                // together so this is safe.
                Ok(self
                    .intern_function_param_or_return_type_tag(tag)?
                    .as_global_arena_ptr())
            })
            .collect::<Result<Vec<_>, TypeError>>()?;

        // Allocate outside the lock to reduce contention. The leak is still
        // bounded to the number of concurrent workers, and therefore is
        // negligible in practice.
        // TODO:
        //   1. For lists, this might not be negligible if the list is large?
        //   2. Consider using alloc_slice_fill_iter.
        let ptr = self.global_arena.alloc_slice_copy(&types);

        // SAFETY: We have just allocated the type list pointer, hence
        // dereferencing it to compute the hash and equality is safe (for
        // transitive pointers as well).
        let ptr = match self.ctx.type_lists.entry(TypeListInternerKey(ptr)) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => *entry.insert(ptr),
        };

        // SAFETY: The allocated pointer is trivially valid until the next
        // maintenance so it is safe to cast its lifetime to the lifetime of
        // the execution guard. If the pointer existed before, it must still
        // be valid (if global arena was flushed, so must have been the map).
        Ok(TypeListRef {
            ptr,
            _guard: PhantomData,
        })
    }

    /// Returns a reference to the type if it has been interned before.
    ///
    /// # Safety precondition
    ///
    /// During global arena reset, types map must be cleared as well.
    fn get_interned_type<'b, Q>(&'b self, key: &Q) -> Option<TypeRef<'b>>
    where
        Q: Hash + Equivalent<TypeInternerKey>,
        'a: 'b,
    {
        self.ctx.types.get(key).map(|entry| {
            // SAFETY: It is safe to cast its lifetime to the lifetime of the
            // execution guard. If the pointer existed before, it must still
            // be valid (during maintenance, if global arena is reset, so is
            // the map).
            TypeRef {
                ptr: *entry.value(),
                _guard: PhantomData,
            }
        })
    }

    /// Returns a reference to the type list if it has been interned before.
    ///
    /// # Safety precondition
    ///
    /// During global arena reset, type lists map must be cleared as well.
    fn get_interned_type_list<'b, Q>(&'b self, key: &Q) -> Option<TypeListRef<'b>>
    where
        Q: Hash + Equivalent<TypeListInternerKey>,
        'a: 'b,
    {
        self.ctx.type_lists.get(key).map(|entry| {
            // SAFETY: It is safe to cast its lifetime to the lifetime of the
            // execution guard. If the pointer existed before, it must still
            // be valid (during maintenance, if global arena is reset, so is
            // the map).
            TypeListRef {
                ptr: *entry.value(),
                _guard: PhantomData,
            }
        })
    }

    /// On interner cache miss, allocates the type in the global arena using
    /// the specified constructor function, and returns a pointer to:
    ///   - Allocated data if the type was not interned before.
    ///   - Existing canonical pointer if the interned type exists (this can
    ///     happen if there is a race condition).
    ///
    /// Returns `Err` if the resulting type would exceed the configured
    /// [`TypeTreeSizeLimits`].
    ///
    /// # Safety precondition
    ///
    /// During global arena reset, types and type list maps must be cleared as
    /// well.
    fn allocate_and_intern_nested_type<'b>(
        &'b self,
        inner_ty: TypeRef<'b>,
        f: impl FnOnce(GlobalArenaPtr<RuntimeTypeInfo>) -> Type,
    ) -> Result<TypeRef<'b>, TypeError>
    where
        'a: 'b,
    {
        let inner_ptr = inner_ty.as_global_arena_ptr();
        let ty = f(inner_ptr);

        // Compute size and check bounds before any allocation.
        let size = compute_type_size(&ty);
        let limits = &self.ctx.type_tree_size_limits;
        if size.depth > limits.max_depth {
            return Err(TypeError::TypeTooDeep {
                depth: size.depth,
                max: limits.max_depth,
            });
        }
        if size.count > limits.max_count {
            return Err(TypeError::TypeTooLarge {
                count: size.count,
                max: limits.max_count,
            });
        }

        // Allocate outside the lock to reduce contention. The leak is still
        // bounded to the number of concurrent workers, and therefore is
        // negligible in practice.
        let ptr = self.global_arena.alloc(RuntimeTypeInfo { ty, size });

        // SAFETY: We have just allocated the type pointer, hence dereferencing
        // it to compute the hash and equality is safe (for transitive pointers
        // as well).
        let ptr = match self.ctx.types.entry(TypeInternerKey(ptr)) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => *entry.insert(ptr),
        };

        // SAFETY: The allocated pointer is trivially valid until the next
        // maintenance so it is safe to cast its lifetime to the lifetime of
        // the execution guard. If the pointer existed before, it must still
        // be valid (if global arena was reset, so must have been the map).
        Ok(TypeRef {
            ptr,
            _guard: PhantomData,
        })
    }

    /// On interner cache miss, allocates the struct type in the global arena
    /// and returns a pointer to:
    ///   - Allocated data if the type was not interned before.
    ///   - Existing canonical pointer if the interned type exists (this can
    ///     happen if there is a race condition).
    ///
    /// Returns `Err` if the resulting type would exceed the configured
    /// [`TypeTreeSizeLimits`].
    ///
    /// # Safety precondition
    ///
    /// During global arena reset, types and type list maps must be cleared as
    /// well.
    fn allocate_and_intern_struct_type<'b>(
        &'b self,
        executable_id_ref: ExecutableIdRef<'b>,
        name_ref: NameRef<'b>,
        ty_args: TypeListRef<'b>,
    ) -> Result<TypeRef<'b>, TypeError>
    where
        'a: 'b,
    {
        let ty = Type::Struct {
            executable_id: executable_id_ref.as_global_arena_ptr(),
            name: name_ref.as_global_arena_ptr(),
            ty_args: ty_args.as_global_arena_ptr(),
        };

        // Compute size and check bounds before any allocation.
        let size = compute_type_size(&ty);
        let limits = &self.ctx.type_tree_size_limits;
        if size.depth > limits.max_depth {
            return Err(TypeError::TypeTooDeep {
                depth: size.depth,
                max: limits.max_depth,
            });
        }
        if size.count > limits.max_count {
            return Err(TypeError::TypeTooLarge {
                count: size.count,
                max: limits.max_count,
            });
        }

        // Allocate outside the lock to reduce contention. The leak is still
        // bounded to the number of concurrent workers, and therefore is
        // negligible in practice.
        let ptr = self.global_arena.alloc(RuntimeTypeInfo { ty, size });

        // SAFETY: We have just allocated the type pointer, hence dereferencing
        // it to compute the hash and equality is safe (for transitive pointers
        // as well).
        let ptr = match self.ctx.types.entry(TypeInternerKey(ptr)) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => *entry.insert(ptr),
        };

        // SAFETY: The allocated pointer is trivially valid until the next
        // maintenance so it is safe to cast its lifetime to the lifetime of
        // the execution guard. If the pointer existed before, it must still
        // be valid (if global arena was reset, so must have been the map).
        Ok(TypeRef {
            ptr,
            _guard: PhantomData,
        })
    }

    /// On interner cache miss, allocates the function type in the global arena
    /// and returns a pointer to:
    ///   - Allocated data if the type was not interned before.
    ///   - Existing canonical pointer if the interned type exists (this can
    ///     happen if there is a race condition).
    ///
    /// Returns `Err` if the resulting type would exceed the configured
    /// [`TypeTreeSizeLimits`].
    ///
    /// # Safety precondition
    ///
    /// During global arena reset, types and type list maps must be cleared as
    /// well.
    fn allocate_and_intern_function_type<'b>(
        &'b self,
        args: TypeListRef<'b>,
        results: TypeListRef<'b>,
        abilities: AbilitySet,
    ) -> Result<TypeRef<'b>, TypeError>
    where
        'a: 'b,
    {
        let ty = Type::Function {
            // SAFETY: By construction, all these global arena pointers are
            // valid. Note that **all** arenas in the global arena pool are
            // reset together so this is safe.
            args: args.as_global_arena_ptr(),
            results: results.as_global_arena_ptr(),
            abilities,
        };

        // Compute size and check bounds before any allocation.
        let size = compute_type_size(&ty);
        let limits = &self.ctx.type_tree_size_limits;
        if size.depth > limits.max_depth {
            return Err(TypeError::TypeTooDeep {
                depth: size.depth,
                max: limits.max_depth,
            });
        }
        if size.count > limits.max_count {
            return Err(TypeError::TypeTooLarge {
                count: size.count,
                max: limits.max_count,
            });
        }

        // Allocate outside the lock to reduce contention. The leak is still
        // bounded to the number of concurrent workers, and therefore is
        // negligible in practice.
        let ptr = self.global_arena.alloc(RuntimeTypeInfo { ty, size });

        // SAFETY: We have just allocated the type pointer, hence dereferencing
        // it to compute the hash and equality is safe (for transitive pointers
        // as well).
        let ptr = match self.ctx.types.entry(TypeInternerKey(ptr)) {
            Entry::Occupied(entry) => *entry.get(),
            Entry::Vacant(entry) => *entry.insert(ptr),
        };

        // SAFETY: The allocated pointer is trivially valid until the next
        // maintenance so it is safe to cast its lifetime to the lifetime of
        // the execution guard. If the pointer existed before, it must still
        // be valid (if global arena was reset, so must have been the map).
        Ok(TypeRef {
            ptr,
            _guard: PhantomData,
        })
    }
}

/// Computes the [`TypeTreeSize`] for a type from its already-interned children.
///
/// # Safety
///
/// All `GlobalArenaPtr` fields inside `ty` must point to live arena allocations
/// (i.e., the arena has not been reset since those pointers were created). This
/// is guaranteed when called from an allocation helper within an `ExecutionGuard`.
fn compute_type_size(ty: &Type) -> TypeTreeSize {
    match ty {
        Type::Bool
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::I256
        | Type::Address
        | Type::Signer
        | Type::TypeParam(_) => TypeTreeSize { count: 1, depth: 1 },

        Type::Vector(inner) | Type::Ref(inner) | Type::RefMut(inner) => {
            // SAFETY: precondition of this function.
            let s = unsafe { inner.as_ref_unchecked() }.size;
            TypeTreeSize {
                count: s.count.saturating_add(1),
                depth: s.depth.saturating_add(1),
            }
        },

        Type::Struct { ty_args, .. } => {
            // SAFETY: precondition of this function.
            let args = unsafe { ty_args.as_ref_unchecked() };
            let max_d = args
                .iter()
                .map(|a| unsafe { a.as_ref_unchecked() }.size.depth)
                .max()
                .unwrap_or(0);
            let sum_c = args.iter().fold(0u32, |acc, a| {
                acc.saturating_add(unsafe { a.as_ref_unchecked() }.size.count)
            });
            TypeTreeSize {
                count: sum_c.saturating_add(1),
                depth: max_d.saturating_add(1),
            }
        },

        Type::Function { args, results, .. } => {
            // SAFETY: precondition of this function.
            let a = unsafe { args.as_ref_unchecked() };
            let r = unsafe { results.as_ref_unchecked() };
            let max_d = a
                .iter()
                .chain(r.iter())
                .map(|x| unsafe { x.as_ref_unchecked() }.size.depth)
                .max()
                .unwrap_or(0);
            let sum_c = a.iter().chain(r.iter()).fold(0u32, |acc, x| {
                acc.saturating_add(unsafe { x.as_ref_unchecked() }.size.count)
            });
            TypeTreeSize {
                count: sum_c.saturating_add(1),
                depth: max_d.saturating_add(1),
            }
        },
    }
}

impl Hash for TypeInternerKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: must be enforced by the caller.
        let info = unsafe { self.0.as_ref_unchecked() };
        match &info.ty {
            Type::Bool => type_discriminant::BOOL.hash(state),
            Type::U8 => type_discriminant::U8.hash(state),
            Type::U16 => type_discriminant::U16.hash(state),
            Type::U32 => type_discriminant::U32.hash(state),
            Type::U64 => type_discriminant::U64.hash(state),
            Type::U128 => type_discriminant::U128.hash(state),
            Type::U256 => type_discriminant::U256.hash(state),
            Type::I8 => type_discriminant::I8.hash(state),
            Type::I16 => type_discriminant::I16.hash(state),
            Type::I32 => type_discriminant::I32.hash(state),
            Type::I64 => type_discriminant::I64.hash(state),
            Type::I128 => type_discriminant::I128.hash(state),
            Type::I256 => type_discriminant::I256.hash(state),
            Type::Address => type_discriminant::ADDRESS.hash(state),
            Type::Signer => type_discriminant::SIGNER.hash(state),
            Type::Vector(ty) => {
                type_discriminant::VECTOR.hash(state);
                Self(*ty).hash(state);
            },
            Type::Ref(ty) => {
                type_discriminant::REFERENCE.hash(state);
                Self(*ty).hash(state);
            },
            Type::RefMut(ty) => {
                type_discriminant::REFERENCE_MUT.hash(state);
                Self(*ty).hash(state);
            },
            Type::Struct {
                executable_id,
                name,
                ty_args,
            } => {
                type_discriminant::STRUCT.hash(state);
                // SAFETY: must be enforced by the caller.
                unsafe {
                    let id = executable_id.as_ref_unchecked();
                    id.address.hash(state);
                    id.name.as_ref_unchecked().hash(state);
                    name.as_ref_unchecked().hash(state);
                };
                TypeListInternerKey(*ty_args).hash(state);
            },

            Type::Function {
                args,
                results,
                abilities,
            } => {
                type_discriminant::FUNCTION.hash(state);
                TypeListInternerKey(*args).hash(state);
                TypeListInternerKey(*results).hash(state);
                abilities.hash(state);
            },

            Type::TypeParam(idx) => {
                type_discriminant::TYPE_PARAM.hash(state);
                idx.hash(state);
            },
        }
    }
}

impl Hash for TypeListInternerKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // SAFETY: must be enforced by the caller.
        let types = unsafe { self.0.as_ref_unchecked() };
        types.len().hash(state);
        for ty in types {
            TypeInternerKey(*ty).hash(state);
        }
    }
}

impl PartialEq for TypeInternerKey {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: must be enforced by the caller.
        let this = unsafe { self.0.as_ref_unchecked() };
        let other = unsafe { other.0.as_ref_unchecked() };

        match &this.ty {
            Type::Bool => matches!(other.ty, Type::Bool),
            Type::U8 => matches!(other.ty, Type::U8),
            Type::U16 => matches!(other.ty, Type::U16),
            Type::U32 => matches!(other.ty, Type::U32),
            Type::U64 => matches!(other.ty, Type::U64),
            Type::U128 => matches!(other.ty, Type::U128),
            Type::U256 => matches!(other.ty, Type::U256),
            Type::I8 => matches!(other.ty, Type::I8),
            Type::I16 => matches!(other.ty, Type::I16),
            Type::I32 => matches!(other.ty, Type::I32),
            Type::I64 => matches!(other.ty, Type::I64),
            Type::I128 => matches!(other.ty, Type::I128),
            Type::I256 => matches!(other.ty, Type::I256),
            Type::Address => matches!(other.ty, Type::Address),
            Type::Signer => matches!(other.ty, Type::Signer),

            Type::Vector(ty) => {
                matches!(&other.ty, Type::Vector(other_ty) if Self(*ty) == Self(*other_ty))
            },
            Type::Ref(ty) => {
                matches!(&other.ty, Type::Ref(other_ty) if Self(*ty) == Self(*other_ty))
            },
            Type::RefMut(ty) => {
                matches!(&other.ty, Type::RefMut(other_ty) if Self(*ty) == Self(*other_ty))
            },

            Type::Struct {
                executable_id,
                name,
                ty_args,
            } => {
                if let Type::Struct {
                    executable_id: other_executable_id,
                    name: other_name,
                    ty_args: other_ty_args,
                } = &other.ty
                {
                    // SAFETY: must be enforced by the caller.
                    unsafe {
                        let id = executable_id.as_ref_unchecked();
                        let other_id = other_executable_id.as_ref_unchecked();
                        let module_name = id.name.as_ref_unchecked();
                        let other_module_name = other_id.name.as_ref_unchecked();
                        let struct_name = name.as_ref_unchecked();
                        let other_struct_name = other_name.as_ref_unchecked();
                        id.address == other_id.address
                            && module_name == other_module_name
                            && struct_name == other_struct_name
                            && TypeListInternerKey(*ty_args) == TypeListInternerKey(*other_ty_args)
                    }
                } else {
                    false
                }
            },

            Type::Function {
                args,
                results,
                abilities,
            } => {
                if let Type::Function {
                    args: other_args,
                    results: other_results,
                    abilities: other_abilities,
                } = &other.ty
                {
                    TypeListInternerKey(*args) == TypeListInternerKey(*other_args)
                        && TypeListInternerKey(*results) == TypeListInternerKey(*other_results)
                        && abilities == other_abilities
                } else {
                    false
                }
            },

            Type::TypeParam(idx) => {
                matches!(&other.ty, Type::TypeParam(other_idx) if idx == other_idx)
            },
        }
    }
}

impl Eq for TypeInternerKey {}

impl PartialEq for TypeListInternerKey {
    fn eq(&self, other: &Self) -> bool {
        // SAFETY: must be enforced by the caller.
        let types = unsafe { self.0.as_ref_unchecked() };
        let other_types = unsafe { other.0.as_ref_unchecked() };

        if types.len() != other_types.len() {
            return false;
        }

        types
            .iter()
            .zip(other_types.iter())
            .all(|(ty, other_ty)| TypeInternerKey(*ty) == TypeInternerKey(*other_ty))
    }
}

impl Eq for TypeListInternerKey {}

impl Hash for LookupKey<'_, TypeTag> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.0 {
            TypeTag::Bool => type_discriminant::BOOL.hash(state),
            TypeTag::U8 => type_discriminant::U8.hash(state),
            TypeTag::U16 => type_discriminant::U16.hash(state),
            TypeTag::U32 => type_discriminant::U32.hash(state),
            TypeTag::U64 => type_discriminant::U64.hash(state),
            TypeTag::U128 => type_discriminant::U128.hash(state),
            TypeTag::U256 => type_discriminant::U256.hash(state),
            TypeTag::I8 => type_discriminant::I8.hash(state),
            TypeTag::I16 => type_discriminant::I16.hash(state),
            TypeTag::I32 => type_discriminant::I32.hash(state),
            TypeTag::I64 => type_discriminant::I64.hash(state),
            TypeTag::I128 => type_discriminant::I128.hash(state),
            TypeTag::I256 => type_discriminant::I256.hash(state),
            TypeTag::Address => type_discriminant::ADDRESS.hash(state),
            TypeTag::Signer => type_discriminant::SIGNER.hash(state),

            TypeTag::Vector(inner) => {
                type_discriminant::VECTOR.hash(state);
                LookupKey(inner.as_ref()).hash(state);
            },

            TypeTag::Struct(struct_tag) => {
                type_discriminant::STRUCT.hash(state);
                struct_tag.address.hash(state);
                struct_tag.module.as_str().hash(state);
                struct_tag.name.as_str().hash(state);
                LookupKey(struct_tag.type_args.as_slice()).hash(state);
            },

            TypeTag::Function(function_tag) => {
                type_discriminant::FUNCTION.hash(state);
                LookupKey(function_tag.args.as_slice()).hash(state);
                LookupKey(function_tag.results.as_slice()).hash(state);
                function_tag.abilities.hash(state);
            },
        }
    }
}

impl Hash for LookupKey<'_, FunctionParamOrReturnTag> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self.0 {
            FunctionParamOrReturnTag::Reference(tag) => {
                type_discriminant::REFERENCE.hash(state);
                LookupKey(tag).hash(state);
            },
            FunctionParamOrReturnTag::MutableReference(tag) => {
                type_discriminant::REFERENCE_MUT.hash(state);
                LookupKey(tag).hash(state);
            },
            FunctionParamOrReturnTag::Value(tag) => {
                // No discriminant is emitted here intentionally. This value
                // is transparent.
                LookupKey(tag).hash(state);
            },
        }
    }
}

impl Hash for LookupKey<'_, [TypeTag]> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for tag in self.0 {
            LookupKey(tag).hash(state);
        }
    }
}

impl Hash for LookupKey<'_, [FunctionParamOrReturnTag]> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.len().hash(state);
        for tag in self.0 {
            LookupKey(tag).hash(state);
        }
    }
}

impl Equivalent<TypeInternerKey> for LookupKey<'_, TypeTag> {
    fn equivalent(&self, key: &TypeInternerKey) -> bool {
        // SAFETY: must be enforced by the caller.
        let key_ty = &unsafe { key.0.as_ref_unchecked() }.ty;

        match self.0 {
            TypeTag::Bool => matches!(key_ty, Type::Bool),
            TypeTag::U8 => matches!(key_ty, Type::U8),
            TypeTag::U16 => matches!(key_ty, Type::U16),
            TypeTag::U32 => matches!(key_ty, Type::U32),
            TypeTag::U64 => matches!(key_ty, Type::U64),
            TypeTag::U128 => matches!(key_ty, Type::U128),
            TypeTag::U256 => matches!(key_ty, Type::U256),
            TypeTag::I8 => matches!(key_ty, Type::I8),
            TypeTag::I16 => matches!(key_ty, Type::I16),
            TypeTag::I32 => matches!(key_ty, Type::I32),
            TypeTag::I64 => matches!(key_ty, Type::I64),
            TypeTag::I128 => matches!(key_ty, Type::I128),
            TypeTag::I256 => matches!(key_ty, Type::I256),
            TypeTag::Address => matches!(key_ty, Type::Address),
            TypeTag::Signer => matches!(key_ty, Type::Signer),

            TypeTag::Vector(tag) => {
                matches!(key_ty, Type::Vector(ty) if LookupKey(tag.as_ref()).equivalent(&TypeInternerKey(*ty)))
            },

            TypeTag::Struct(struct_tag) => {
                if let Type::Struct {
                    executable_id,
                    name,
                    ty_args,
                } = key_ty
                {
                    // SAFETY: must be enforced by the caller.
                    unsafe {
                        let id = executable_id.as_ref_unchecked();
                        id.address == struct_tag.address
                            && id.name.as_ref_unchecked() == struct_tag.module.as_str()
                            && name.as_ref_unchecked() == struct_tag.name.as_str()
                            && LookupKey(struct_tag.type_args.as_slice())
                                .equivalent(&TypeListInternerKey(*ty_args))
                    }
                } else {
                    false
                }
            },

            TypeTag::Function(function_tag) => {
                if let Type::Function {
                    args,
                    results,
                    abilities,
                } = key_ty
                {
                    &function_tag.abilities == abilities
                        && LookupKey(function_tag.args.as_slice())
                            .equivalent(&TypeListInternerKey(*args))
                        && LookupKey(function_tag.results.as_slice())
                            .equivalent(&TypeListInternerKey(*results))
                } else {
                    false
                }
            },
        }
    }
}

impl Equivalent<TypeInternerKey> for LookupKey<'_, FunctionParamOrReturnTag> {
    fn equivalent(&self, key: &TypeInternerKey) -> bool {
        match self.0 {
            FunctionParamOrReturnTag::Reference(inner_tag) => {
                // SAFETY: must be enforced by the caller.
                if let Type::Ref(ty) = unsafe { &key.0.as_ref_unchecked().ty } {
                    LookupKey(inner_tag).equivalent(&TypeInternerKey(*ty))
                } else {
                    false
                }
            },
            FunctionParamOrReturnTag::MutableReference(inner_tag) => {
                // SAFETY: must be enforced by the caller.
                if let Type::RefMut(ty) = unsafe { &key.0.as_ref_unchecked().ty } {
                    LookupKey(inner_tag).equivalent(&TypeInternerKey(*ty))
                } else {
                    false
                }
            },
            FunctionParamOrReturnTag::Value(inner_tag) => {
                LookupKey(inner_tag).equivalent(&TypeInternerKey(key.0))
            },
        }
    }
}

/// Checks whether a slice of tags is equivalent to an interned type list key.
///
/// Extracted to eliminate duplication between the two
/// `Equivalent<TypeListInternerKey>` impls.
///
/// **Compatibility note**: the inner element type used in the comparison is
/// `TypeInternerKey(GlobalArenaPtr<RuntimeTypeInfo>)`, which matches the
/// current pointer representation in `TypeListInternerKey`.
fn slice_equivalent_to_type_list<T>(tags: &[T], key: &TypeListInternerKey) -> bool
where
    for<'a> LookupKey<'a, T>: Equivalent<TypeInternerKey>,
{
    // SAFETY: must be enforced by the caller.
    let tys = unsafe { key.0.as_ref_unchecked() };
    tags.len() == tys.len()
        && tags
            .iter()
            .zip(tys.iter())
            .all(|(tag, ty)| LookupKey(tag).equivalent(&TypeInternerKey(*ty)))
}

impl Equivalent<TypeListInternerKey> for LookupKey<'_, [TypeTag]> {
    fn equivalent(&self, key: &TypeListInternerKey) -> bool {
        slice_equivalent_to_type_list(self.0, key)
    }
}

impl Equivalent<TypeListInternerKey> for LookupKey<'_, [FunctionParamOrReturnTag]> {
    fn equivalent(&self, key: &TypeListInternerKey) -> bool {
        slice_equivalent_to_type_list(self.0, key)
    }
}

/// Returns a [`TypeRef`] backed by statically allocated types (primitives).
#[inline(always)]
fn static_type_ref(ty: &'static RuntimeTypeInfo) -> TypeRef<'static> {
    TypeRef {
        ptr: GlobalArenaPtr::from_static(ty),
        _guard: PhantomData,
    }
}

/// Returns a [`TypeListRef`] backed by a statically allocated type list.
#[inline(always)]
fn static_type_list_ref(list: &'static [GlobalArenaPtr<RuntimeTypeInfo>]) -> TypeListRef<'static> {
    TypeListRef {
        ptr: GlobalArenaPtr::from_static(list),
        _guard: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        alloc::{GlobalArenaPool, GlobalArenaPtr},
        context::executable_ids::ExecutableId,
    };
    use dashmap::Equivalent;
    use move_core_types::{
        ability::AbilitySet,
        account_address::AccountAddress,
        ident_str,
        language_storage::{FunctionParamOrReturnTag, FunctionTag, StructTag, TypeTag},
    };
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };

    fn hash_of<H: Hash>(h: &H) -> u64 {
        let mut s = DefaultHasher::new();
        h.hash(&mut s);
        s.finish()
    }

    /// Returns all primitive types.
    fn all_primitives() -> Vec<(TypeTag, &'static RuntimeTypeInfo)> {
        vec![
            (TypeTag::Bool, &BOOL),
            (TypeTag::U8, &U8),
            (TypeTag::U16, &U16),
            (TypeTag::U32, &U32),
            (TypeTag::U64, &U64),
            (TypeTag::U128, &U128),
            (TypeTag::U256, &U256),
            (TypeTag::I8, &I8),
            (TypeTag::I16, &I16),
            (TypeTag::I32, &I32),
            (TypeTag::I64, &I64),
            (TypeTag::I128, &I128),
            (TypeTag::I256, &I256),
            (TypeTag::Address, &ADDRESS),
            (TypeTag::Signer, &SIGNER),
        ]
    }

    #[test]
    fn test_primitive_types_hash() {
        for (tag, ty_static) in all_primitives() {
            let key = TypeInternerKey(GlobalArenaPtr::from_static(ty_static));
            let lookup_key = LookupKey(&tag);
            assert_eq!(hash_of(&key), hash_of(&lookup_key));
        }
    }

    #[test]
    fn test_primitive_types_equivalence() {
        let pairs = all_primitives();
        for (i, (tag_i, _)) in pairs.iter().enumerate() {
            for (j, (_, ty_j)) in pairs.iter().enumerate() {
                let key = TypeInternerKey(GlobalArenaPtr::from_static(ty_j));
                let lookup_key = LookupKey(tag_i);
                if i == j {
                    assert!(lookup_key.equivalent(&key));
                } else {
                    assert!(!lookup_key.equivalent(&key));
                }
            }
        }
    }

    #[test]
    fn test_primitive_types_equality() {
        let pairs = all_primitives();
        for (i, (_, ty_i)) in pairs.iter().enumerate() {
            for (j, (_, ty_j)) in pairs.iter().enumerate() {
                let key_i = TypeInternerKey(GlobalArenaPtr::from_static(ty_i));
                let key_j = TypeInternerKey(GlobalArenaPtr::from_static(ty_j));
                if i == j {
                    assert!(key_i == key_j);
                } else {
                    assert!(key_i != key_j);
                }
            }
        }
    }

    // ── 3d. Hash consistency for Vector (using a local arena) ─────────────────

    #[test]
    fn test_vector_types_hash_and_equivalence() {
        let pool = GlobalArenaPool::with_num_arenas(1);

        let vec_bool = pool.lock_arena(0).alloc(RuntimeTypeInfo {
            ty: Type::Vector(GlobalArenaPtr::from_static(&BOOL)),
            size: TypeTreeSize { count: 2, depth: 2 },
        });
        let vec_bool_tag = TypeTag::Vector(Box::new(TypeTag::Bool));

        let vec_vec_bool = pool.lock_arena(0).alloc(RuntimeTypeInfo {
            ty: Type::Vector(vec_bool),
            size: TypeTreeSize { count: 3, depth: 3 },
        });
        let vec_vec_bool_tag = TypeTag::Vector(Box::new(vec_bool_tag.clone()));

        assert_eq!(
            hash_of(&TypeInternerKey(vec_bool)),
            hash_of(&LookupKey(&vec_bool_tag))
        );
        assert!(LookupKey(&vec_bool_tag).equivalent(&TypeInternerKey(vec_bool)));

        assert_eq!(
            hash_of(&TypeInternerKey(vec_vec_bool)),
            hash_of(&LookupKey(&vec_vec_bool_tag))
        );
        assert!(LookupKey(&vec_vec_bool_tag).equivalent(&TypeInternerKey(vec_vec_bool)));

        let lookup_key = LookupKey(&TypeTag::Vector(Box::new(TypeTag::U8)));
        assert!(!lookup_key.equivalent(&TypeInternerKey(vec_bool)));
    }

    #[test]
    fn test_type_list_hash_and_equivalence() {
        let empty_key = TypeListInternerKey(GlobalArenaPtr::from_static(&EMPTY_TYPE_LIST));
        let empty_lookup = LookupKey::<[TypeTag]>(&[]);

        assert_eq!(hash_of(&empty_key), hash_of(&empty_lookup));
        assert!(empty_lookup.equivalent(&empty_key));

        let pool = GlobalArenaPool::with_num_arenas(1);

        let bool = GlobalArenaPtr::from_static(&BOOL);
        let u8 = GlobalArenaPtr::from_static(&U8);
        let list_bool = pool.lock_arena(0).alloc_slice_copy(&[bool]);
        let list_bool_u8 = pool.lock_arena(0).alloc_slice_copy(&[bool, u8]);
        let list_u8_bool = pool.lock_arena(0).alloc_slice_copy(&[u8, bool]);

        let list_bool_key = TypeListInternerKey(list_bool);
        let list_bool_u8_key = TypeListInternerKey(list_bool_u8);
        let list_u8_bool_key = TypeListInternerKey(list_u8_bool);

        let list_bool_lookup = LookupKey::<[TypeTag]>(&[TypeTag::Bool]);
        let list_u8_lookup = LookupKey::<[TypeTag]>(&[TypeTag::U8]);
        let list_bool_u8_lookup = LookupKey::<[TypeTag]>(&[TypeTag::Bool, TypeTag::U8]);
        let list_u8_bool_lookup = LookupKey::<[TypeTag]>(&[TypeTag::U8, TypeTag::Bool]);

        assert_eq!(hash_of(&list_bool_key), hash_of(&list_bool_lookup));
        assert!(list_bool_lookup.equivalent(&list_bool_key));

        let duplicate_list_bool_key =
            TypeListInternerKey(pool.lock_arena(0).alloc_slice_copy(&[bool]));
        assert!(list_bool_key == duplicate_list_bool_key);

        assert_eq!(hash_of(&list_bool_u8_key), hash_of(&list_bool_u8_lookup));
        assert!(list_bool_u8_lookup.equivalent(&list_bool_u8_key));

        assert_eq!(hash_of(&list_u8_bool_key), hash_of(&list_u8_bool_lookup));
        assert!(list_u8_bool_lookup.equivalent(&list_u8_bool_key));

        assert_ne!(hash_of(&list_bool_u8_lookup), hash_of(&list_u8_bool_lookup));
        assert_ne!(hash_of(&list_bool_u8_key), hash_of(&list_u8_bool_key));
        assert!(!list_bool_u8_lookup.equivalent(&list_u8_bool_key));

        assert!(!list_bool_lookup.equivalent(&empty_key));
        assert!(!list_bool_u8_lookup.equivalent(&list_bool_key));
        assert!(!list_u8_lookup.equivalent(&list_bool_key));
    }

    #[test]
    fn test_function_param_or_return_value_transparent_hash() {
        let bool = FunctionParamOrReturnTag::Value(TypeTag::Bool);
        assert_eq!(
            hash_of(&LookupKey(&bool)),
            hash_of(&LookupKey(&TypeTag::Bool))
        );

        let bool_key = TypeInternerKey(GlobalArenaPtr::from_static(&BOOL));
        assert!(LookupKey(&bool).equivalent(&bool_key));
    }

    #[test]
    fn test_function_param_or_return_reference_hash_and_equivalence() {
        let pool = GlobalArenaPool::with_num_arenas(1);
        let ref_bool = pool.lock_arena(0).alloc(RuntimeTypeInfo {
            ty: Type::Ref(GlobalArenaPtr::from_static(&BOOL)),
            size: TypeTreeSize { count: 2, depth: 2 },
        });
        let ref_bool_copy = pool.lock_arena(0).alloc(RuntimeTypeInfo {
            ty: Type::Ref(GlobalArenaPtr::from_static(&BOOL)),
            size: TypeTreeSize { count: 2, depth: 2 },
        });
        let ref_mut_bool = pool.lock_arena(0).alloc(RuntimeTypeInfo {
            ty: Type::RefMut(GlobalArenaPtr::from_static(&BOOL)),
            size: TypeTreeSize { count: 2, depth: 2 },
        });

        let ref_tag = FunctionParamOrReturnTag::Reference(TypeTag::Bool);
        let ref_mut_tag = FunctionParamOrReturnTag::MutableReference(TypeTag::Bool);

        let ref_bool_key = TypeInternerKey(ref_bool);
        let ref_bool_key_copy = TypeInternerKey(ref_bool_copy);
        let ref_mut_bool_key = TypeInternerKey(ref_mut_bool);
        let ref_bool_lookup = LookupKey(&ref_tag);
        let ref_mut_bool_lookup = LookupKey(&ref_mut_tag);

        assert_ne!(
            hash_of(&ref_bool_lookup),
            hash_of(&LookupKey(&TypeTag::Bool))
        );
        assert_ne!(hash_of(&ref_mut_bool_lookup), hash_of(&ref_bool_key));
        assert_eq!(hash_of(&ref_bool_lookup), hash_of(&ref_bool_key));

        assert!(ref_bool_lookup.equivalent(&ref_bool_key));
        assert!(!ref_bool_lookup.equivalent(&ref_mut_bool_key));
        assert!(!ref_mut_bool_lookup.equivalent(&ref_bool_key));

        assert!(ref_bool != ref_bool_copy);
        assert!(ref_bool_key == ref_bool_key_copy);
        assert!(ref_bool_key != ref_mut_bool_key);
    }

    #[test]
    fn test_type_param_hash_and_equivalence() {
        let pool = GlobalArenaPool::with_num_arenas(1);
        let param0 = pool.lock_arena(0).alloc(RuntimeTypeInfo {
            ty: Type::TypeParam(0),
            size: TypeTreeSize { count: 1, depth: 1 },
        });
        let param0_copy = pool.lock_arena(0).alloc(RuntimeTypeInfo {
            ty: Type::TypeParam(0),
            size: TypeTreeSize { count: 1, depth: 1 },
        });
        let param1 = pool.lock_arena(0).alloc(RuntimeTypeInfo {
            ty: Type::TypeParam(1),
            size: TypeTreeSize { count: 1, depth: 1 },
        });

        let key0 = TypeInternerKey(param0);
        let key0_copy = TypeInternerKey(param0_copy);
        let key1 = TypeInternerKey(param1);

        assert!(param0 != param0_copy);
        assert_ne!(hash_of(&param0), hash_of(&param0_copy));

        assert!(key0 == key0_copy);
        assert_eq!(hash_of(&key0), hash_of(&key0_copy));

        assert!(key0 != key1);
        assert_ne!(hash_of(&key0), hash_of(&key1));

        let bool_lookup = LookupKey(&TypeTag::Bool);
        assert_ne!(hash_of(&key0), hash_of(&bool_lookup));
        assert!(!bool_lookup.equivalent(&key0));
    }

    #[test]
    fn test_function_param_tag_list_hash_and_equivalence() {
        let pool = GlobalArenaPool::with_num_arenas(1);

        let u8 = GlobalArenaPtr::from_static(&U8);
        let ref_u8 = pool.lock_arena(0).alloc(RuntimeTypeInfo {
            ty: Type::Ref(u8),
            size: TypeTreeSize { count: 2, depth: 2 },
        });
        let bool = GlobalArenaPtr::from_static(&BOOL);
        let list = pool.lock_arena(0).alloc_slice_copy(&[ref_u8, bool]);

        let list_key = TypeListInternerKey(list);
        let list_lookup = LookupKey::<[FunctionParamOrReturnTag]>(&[
            FunctionParamOrReturnTag::Reference(TypeTag::U8),
            FunctionParamOrReturnTag::Value(TypeTag::Bool),
        ]);

        assert_eq!(hash_of(&list_key), hash_of(&list_lookup));
        assert!(list_lookup.equivalent(&list_key),);

        let empty_key = TypeListInternerKey(GlobalArenaPtr::from_static(&EMPTY_TYPE_LIST));
        assert!(!list_lookup.equivalent(&empty_key));
    }

    #[test]
    fn test_struct_type_hash_and_equivalence() {
        let pool = GlobalArenaPool::with_num_arenas(1);

        let name = pool.lock_arena(0).alloc_str("foo");
        let executable_id = pool.lock_arena(0).alloc(ExecutableId {
            address: AccountAddress::ONE,
            name,
        });

        let name = pool.lock_arena(0).alloc_str("Bar");
        let struct_type = pool.lock_arena(0).alloc(RuntimeTypeInfo {
            ty: Type::Struct {
                executable_id,
                name,
                ty_args: GlobalArenaPtr::from_static(&EMPTY_TYPE_LIST),
            },
            size: TypeTreeSize { count: 1, depth: 1 },
        });

        let tag = StructTag {
            address: AccountAddress::ONE,
            module: ident_str!("foo").to_owned(),
            name: ident_str!("Bar").to_owned(),
            type_args: vec![],
        };
        let key = TypeInternerKey(struct_type);
        let lookup_key = LookupKey(&TypeTag::Struct(Box::new(tag.clone())));
        assert_eq!(hash_of(&key), hash_of(&lookup_key));
        assert!(lookup_key.equivalent(&key),);

        let mut wrong_tag_1 = tag.clone();
        wrong_tag_1.address = AccountAddress::TWO;

        let mut wrong_tag_2 = tag.clone();
        wrong_tag_2.module = ident_str!("bar").to_owned();

        let mut wrong_tag_3 = tag.clone();
        wrong_tag_3.name = ident_str!("Foo").to_owned();

        let mut wrong_tag_4 = tag.clone();
        wrong_tag_4.type_args = vec![TypeTag::Bool];

        for wrong_tag in [wrong_tag_1, wrong_tag_2, wrong_tag_3, wrong_tag_4] {
            let wrong_lookup_key = LookupKey(&TypeTag::Struct(Box::new(wrong_tag)));
            assert_eq!(hash_of(&key), hash_of(&lookup_key));
            assert!(!wrong_lookup_key.equivalent(&key));
        }
    }

    #[test]
    fn test_function_type_interner_key_hash_and_equivalence() {
        let pool = GlobalArenaPool::with_num_arenas(1);

        let func = pool.lock_arena(0).alloc(RuntimeTypeInfo {
            ty: Type::Function {
                args: GlobalArenaPtr::from_static(&EMPTY_TYPE_LIST),
                results: GlobalArenaPtr::from_static(&EMPTY_TYPE_LIST),
                abilities: AbilitySet::EMPTY,
            },
            size: TypeTreeSize { count: 1, depth: 1 },
        });

        let empty_key = TypeInternerKey(func);
        let empty_lookup_key = LookupKey(&TypeTag::Function(Box::new(FunctionTag {
            args: vec![],
            results: vec![],
            abilities: AbilitySet::EMPTY,
        })));

        assert_eq!(hash_of(&empty_key), hash_of(&empty_lookup_key));
        assert!(empty_lookup_key.equivalent(&empty_key));

        // Different abilities must produce a different hash.
        let func = pool.lock_arena(0).alloc(RuntimeTypeInfo {
            ty: Type::Function {
                args: GlobalArenaPtr::from_static(&EMPTY_TYPE_LIST),
                results: GlobalArenaPtr::from_static(&EMPTY_TYPE_LIST),
                abilities: AbilitySet::ALL,
            },
            size: TypeTreeSize { count: 1, depth: 1 },
        });
        let all_key = TypeInternerKey(func);
        let all_lookup_key = LookupKey(&TypeTag::Function(Box::new(FunctionTag {
            args: vec![],
            results: vec![],
            abilities: AbilitySet::ALL,
        })));

        assert_eq!(hash_of(&all_key), hash_of(&all_lookup_key));
        assert!(all_lookup_key.equivalent(&all_key));
        assert_ne!(hash_of(&empty_key), hash_of(&all_key));
        assert!(!empty_lookup_key.equivalent(&all_key));
        assert!(!all_lookup_key.equivalent(&empty_key));
    }
}
