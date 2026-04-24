// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    types::{InternedType, Type},
    Function,
};
use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};
use move_core_types::account_address::AccountAddress;
use parking_lot::Mutex;
use shared_dsa::UnorderedMap;

/// Identifies an executable (module or script) by its address and name.
///   - For modules, constructed from module address and name.
///   - For scripts: TODO
///
/// # Safety
///
/// Must be created from a valid global arena pointer to executable's name.
pub struct ExecutableId {
    address: AccountAddress,
    name: GlobalArenaPtr<str>,
}

impl ExecutableId {
    /// Creates a new executable ID.
    ///
    /// # Safety
    ///
    /// The caller must ensure name points to a valid, live arena allocation.
    pub unsafe fn new(address: AccountAddress, name: GlobalArenaPtr<str>) -> Self {
        Self { address, name }
    }

    /// Returns the account address of this executable.
    pub fn address(&self) -> &AccountAddress {
        &self.address
    }

    /// Returns the arena pointer to the name.
    pub fn name(&self) -> GlobalArenaPtr<str> {
        self.name
    }
}

// ================================================================================================
// Executable and supporting types
// ================================================================================================

/// Struct type metadata in an executable.
pub struct StructType {
    /// Struct type signature. Invariant: stored type is always
    /// [`Type::Struct`].
    ty: InternedType,
}

impl StructType {
    /// Creates a new struct type entry.
    pub fn new(ty: InternedType) -> Self {
        Self { ty }
    }

    /// Returns the underlying type pointer.
    pub fn ty(&self) -> InternedType {
        self.ty
    }
}

/// Enum type metadata in an executable.
pub struct EnumType {
    /// Enum type signature. Invariant: stored type is always
    /// [`Type::Enum`].
    ty: InternedType,
    /// Per-variant field types, indexed by variant tag.
    #[allow(dead_code)]
    variants: ExecutableArenaPtr<[VariantFields]>,
}

impl EnumType {
    /// Creates a new enum type entry.
    pub fn new(ty: InternedType, variants: ExecutableArenaPtr<[VariantFields]>) -> Self {
        Self { ty, variants }
    }

    /// Returns the underlying type pointer.
    pub fn ty(&self) -> InternedType {
        self.ty
    }
}

/// Field types for a single enum variant.
#[derive(Copy, Clone)]
pub struct VariantFields {
    #[allow(dead_code)]
    fields: ExecutableArenaPtr<[InternedType]>,
}

impl VariantFields {
    /// Creates a new variant fields entry.
    pub fn new(fields: ExecutableArenaPtr<[InternedType]>) -> Self {
        Self { fields }
    }
}

/// A loaded executable (from module or script).
pub struct Executable {
    data: ExecutableData,

    /// Arena where data is allocated for this executable. **Must** be the
    /// last field so that it is dropped after any data structure that holds
    /// pointers into it.
    #[allow(dead_code)]
    arena: Mutex<ExecutableArena>,
}

struct ExecutableData {
    /// Executable ID which uniquely identifies this executable.
    id: GlobalArenaPtr<ExecutableId>,
    /// Non-generic struct definitions.
    structs: UnorderedMap<GlobalArenaPtr<str>, StructType>,
    /// Non-generic enum definitions.
    #[allow(dead_code)]
    enums: UnorderedMap<GlobalArenaPtr<str>, EnumType>,
    /// Non-generic functions.
    functions: UnorderedMap<GlobalArenaPtr<str>, ExecutableArenaPtr<Function>>,
}

impl Executable {
    /// Creates a new executable.
    // TODO: the current constructor accepts pre-populated maps whose values
    // are pointers into `arena` (and into the global arena), but there is
    // nothing at the type level tying each pointer to the arena it came
    // from. Consider replacing this with a builder API that owns the
    // `ExecutableArena` internally and exposes `add_struct`/`add_enum`/
    // `add_function` entry points, so external callers cannot smuggle in
    // pointers backed by a different arena.
    pub fn new(
        id: GlobalArenaPtr<ExecutableId>,
        structs: UnorderedMap<GlobalArenaPtr<str>, StructType>,
        enums: UnorderedMap<GlobalArenaPtr<str>, EnumType>,
        functions: UnorderedMap<GlobalArenaPtr<str>, ExecutableArenaPtr<Function>>,
        arena: ExecutableArena,
    ) -> Box<Self> {
        Box::new(Self {
            data: ExecutableData {
                id,
                structs,
                enums,
                functions,
            },
            arena: Mutex::new(arena),
        })
    }

    /// Returns a non-generic function from this executable. Returns [`None`]
    /// if such function does not exist.
    pub fn get_function(&self, name: GlobalArenaPtr<str>) -> Option<&Function> {
        self.data.functions.get(&name).map(|ptr| {
            // SAFETY: Because executable is alive, all its allocations are
            // still valid.
            unsafe { ptr.as_ref_unchecked() }
        })
    }

    /// Returns a non-generic struct type from this executable. Returns [`None`]
    /// if such struct does not exist.
    pub fn get_struct(&self, name: GlobalArenaPtr<str>) -> Option<&Type> {
        self.data.structs.get(&name).map(|st| {
            // SAFETY: Types must be still valid
            unsafe { st.ty.as_ref_unchecked() }
        })
    }

    /// Returns the executable ID pointer.
    pub fn id(&self) -> GlobalArenaPtr<ExecutableId> {
        self.data.id
    }
}
