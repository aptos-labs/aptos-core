// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    types::{InternedType, Type},
    Function,
};
use mono_move_alloc::{
    ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr, LeakedBoxPtr, VersionedLeakedBoxPtr,
};
use move_core_types::account_address::AccountAddress;
use parking_lot::Mutex;
use shared_dsa::UnorderedMap;
use std::sync::{Arc, OnceLock};

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
// Executable cache slot and mandatory dependencies
// ================================================================================================

/// Stable slot pointer for an executable in the cache. May be empty if the
/// executable has not yet been cached.
pub type ExecutableSlot = LeakedBoxPtr<VersionedLeakedBoxPtr<Executable>>;

/// What a loaded executable says about its mandatory dependencies, keyed by
/// the loading policy that built it. Always excludes self.
pub enum MandatoryDependencies {
    /// This executable has been loaded lazily. There are no mandatory
    /// dependencies.
    None,
    /// This executable has been loaded under package policy. The list includes
    /// every other member of the executable's package; together with self they
    /// form a package bundle.
    Package(Arc<[ExecutableSlot]>),
    /// This executable has been loaded under lazy policy with transitive
    /// structs. The inner value is **not set** when the executable was loaded
    /// as a sub-member of another executable's closure (its own closure was
    /// not computed). It is **set once** when the executable is loaded as a
    /// target and its transitive-struct closure is computed.
    TransitiveStructClosure(OnceLock<Box<[ExecutableSlot]>>),
}

impl MandatoryDependencies {
    pub fn transitive_unset() -> Self {
        Self::TransitiveStructClosure(OnceLock::new())
    }

    pub fn transitive_from_slots(slots: Box<[ExecutableSlot]>) -> Self {
        let cell = OnceLock::new();
        let _ = cell.set(slots);
        Self::TransitiveStructClosure(cell)
    }

    /// Slots of the other modules this executable pins. Returns
    /// [`Some`] for `None`/`Package` (the slice is always known), and for
    /// `TransitiveStructClosure` only once the closure has been computed.
    /// Returns [`None`] for a `TransitiveStructClosure` whose closure has
    /// not yet been filled in — callers must compute it and call
    /// [`MandatoryDependencies::set_struct_closure`] before using it.
    pub fn slots(&self) -> Option<&[ExecutableSlot]> {
        match self {
            Self::None => Some(&[]),
            Self::Package(slots) => Some(slots),
            Self::TransitiveStructClosure(cell) => cell.get().map(|boxed| &boxed[..]),
        }
    }

    /// Installs the closure on a `TransitiveStructClosure` variant. No-op
    /// if already set, or if the variant is `None` / `Package`. Since
    /// struct-def closures are a deterministic function of the module's
    /// bytecode, any two concurrent computers produce identical slot
    /// lists, so losers on a race drop their value on the floor.
    pub fn set_struct_closure(&self, slots: Box<[ExecutableSlot]>) {
        if let Self::TransitiveStructClosure(cell) = self {
            let _ = cell.set(slots);
        }
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
    /// Deterministic load cost for this executable.
    cost: u64,
    /// Slots of every other module in this executable's mandatory-dependency
    /// set. Slots are aliases of the leaked-box slots owned by the cache;
    /// dropping these aliases does not free any slot.
    mandatory_dependencies: MandatoryDependencies,
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
        cost: u64,
        mandatory_dependencies: MandatoryDependencies,
        structs: UnorderedMap<GlobalArenaPtr<str>, StructType>,
        enums: UnorderedMap<GlobalArenaPtr<str>, EnumType>,
        functions: UnorderedMap<GlobalArenaPtr<str>, ExecutableArenaPtr<Function>>,
        arena: ExecutableArena,
    ) -> Box<Self> {
        Box::new(Self {
            data: ExecutableData {
                id,
                cost,
                mandatory_dependencies,
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

    /// Returns the deterministic load cost for this executable.
    pub fn cost(&self) -> u64 {
        self.data.cost
    }

    /// Returns the slots of every other module in this executable's
    /// mandatory dependencies.
    pub fn mandatory_dependencies(&self) -> &MandatoryDependencies {
        &self.data.mandatory_dependencies
    }
}
