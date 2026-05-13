// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Loaded module — what the module cache stores. Stores the polymorphic IR
//! with the lowered monomorphic functions and generic function instantiations.

use crate::context::ExecutionGuard;
use anyhow::{anyhow, bail};
use mono_move_alloc::{LeakedBoxPtr, VersionedLeakedBoxPtr};
use mono_move_core::{
    interner::{InternedIdentifier, InternedModuleId},
    Function, FunctionPtr,
};
use shared_dsa::UnorderedMap;
use specializer::{FunctionIR, ModuleIR};
use std::sync::{Arc, OnceLock};

/// Stable cache slot for a loaded module. The slot's identifier is fixed at
/// creation time (from the cache key), while the content may be empty until
/// the module is installed.
pub struct ModuleSlot {
    id: InternedModuleId,
    versions: VersionedLeakedBoxPtr<LoadedModule>,
}

impl ModuleSlot {
    /// Creates an empty slot for the specified module ID.
    pub fn new(id: InternedModuleId) -> Self {
        Self {
            id,
            versions: VersionedLeakedBoxPtr::new(),
        }
    }

    /// Returns the module ID this slot is keyed by.
    pub fn id(&self) -> InternedModuleId {
        self.id
    }

    /// Reads the current loaded module behind this slot. If slot is not set,
    /// returns [`None`].
    pub fn get<'guard>(&self, _guard: &'guard ExecutionGuard<'_>) -> Option<&'guard LoadedModule> {
        // SAFETY: while a guard is held, maintenance cannot run, so the
        // pointer is alive for and not null.
        unsafe { self.versions.load().map(|p| p.as_ref_unchecked()) }
    }

    /// Returns the raw pointer to the module if set, and [`None`] otherwise.
    pub fn get_ptr(&self) -> Option<LeakedBoxPtr<LoadedModule>> {
        self.versions.load()
    }

    /// Sets the slot content if empty. See [`VersionedLeakedBoxPtr::init`]
    /// for race semantics.
    pub fn init(&self, ptr: LeakedBoxPtr<LoadedModule>) -> Result<(), LeakedBoxPtr<LoadedModule>> {
        self.versions.init(ptr)
    }

    /// Atomically empties the slot, returning the previous content if any.
    pub fn clear(&self) -> Option<LeakedBoxPtr<LoadedModule>> {
        self.versions.clear()
    }
}

/// Stable slot pointer for a loaded module in the cache. May be empty if the
/// module has not yet been cached.
pub type LoadedModuleSlot = LeakedBoxPtr<ModuleSlot>;

/// What a loaded module says about its mandatory dependencies, keyed by the
/// loading policy that built it.
#[derive(Clone)]
pub enum ModuleMandatoryDependencies {
    /// Cell that may be filled with slots at a later time. Used by all lazy
    /// (LL and EL) policies, and for module loads for lowering (shallow).
    ///
    /// # Invariants
    ///   1. Under LL the cell is always empty.
    ///   2. For shallow layout loads, the cell is created empty.
    ///   3. Under EL the loader fills it for module M once MS(M) has been
    ///      computed. Shallow side-loads stay with an unset cell until they require lowering.
    ///   4. Filled entries always include self.
    Lazy(OnceLock<Arc<[LoadedModuleSlot]>>),
    /// Every member of the same package as the owning module. Includes the
    /// owning module itself. Used by package loading (PL) policy.
    Package(Arc<[LoadedModuleSlot]>),
}

impl ModuleMandatoryDependencies {
    /// Slots of the modules this module loaded together with.
    pub fn slots(&self) -> &[LoadedModuleSlot] {
        match self {
            Self::Lazy(cell) => cell.get().map(|s| s.as_ref()).unwrap_or(&[]),
            Self::Package(slots) => slots,
        }
    }

    /// Returns the cell for lazy mandatory dependencies.
    pub fn as_lazy(&self) -> anyhow::Result<&OnceLock<Arc<[LoadedModuleSlot]>>> {
        let Self::Lazy(cell) = self else {
            bail!("Mandatory dependencies must always be lazy");
        };
        Ok(cell)
    }

    /// Returns empty mandatory dependencies for lazy module loads. This does
    /// not include self.
    pub fn lazy_unset() -> Self {
        Self::Lazy(OnceLock::new())
    }

    /// Returns mandatory dependencies for the package. Always includes self.
    pub fn package(package_slots: Vec<LoadedModuleSlot>) -> Self {
        Self::Package(Arc::from(package_slots))
    }
}

pub struct FunctionSlot {
    pub function: FunctionPtr,
    pub mandatory_dependencies: Vec<LoadedModuleSlot>,
}

impl FunctionSlot {
    /// Returns a new slot owning the monomorphic function with its mandatory
    /// dependencies.
    pub fn new(function: Function, mandatory_dependencies: Vec<LoadedModuleSlot>) -> Self {
        Self {
            function: FunctionPtr::new(Box::new(function)),
            mandatory_dependencies,
        }
    }
}

/// A loaded module: polymorphic IR and lazily lowered monomorphic functions.
pub struct LoadedModule {
    /// Polymorphic stackless IR.
    ir: ModuleIR,
    /// Deterministic load cost recorded at insertion time.
    cost: u64,
    /// Mandatory-dependency descriptor produced by the loader's policy. These
    /// are all modules that have to be loaded together with this module.
    mandatory_dependencies: ModuleMandatoryDependencies,
    /// Per-name slot for the lazily lowered monomorphic functions.
    functions: UnorderedMap<InternedIdentifier, OnceLock<FunctionSlot>>,
    /// Maps function's name to its index in file format (to query its IR).
    function_indices: UnorderedMap<InternedIdentifier, usize>,
}

impl LoadedModule {
    pub fn new(
        ir: ModuleIR,
        cost: u64,
        mandatory_dependencies: ModuleMandatoryDependencies,
    ) -> Box<Self> {
        let mut functions = UnorderedMap::with_capacity(ir.functions.len());
        let mut function_indices = UnorderedMap::with_capacity(ir.functions.len());

        for (idx, func_ir) in ir.functions.iter().enumerate() {
            match func_ir {
                Some(func_ir) => {
                    let name = ir.module.interned_identifier_at(func_ir.name_idx);
                    functions.insert(name, OnceLock::new());
                    function_indices.insert(name, idx);
                },
                None => {
                    // TODO: For natives we also need to add a function?
                },
            }
        }
        Box::new(Self {
            ir,
            cost,
            mandatory_dependencies,
            functions,
            function_indices,
        })
    }

    /// Returns the polymorphic stackless IR.
    pub fn ir(&self) -> &ModuleIR {
        &self.ir
    }

    /// Returns the mandatory-dependency descriptor.
    pub fn mandatory_dependencies(&self) -> &ModuleMandatoryDependencies {
        &self.mandatory_dependencies
    }

    /// Returns interned module ID of this module.
    pub fn id(&self) -> InternedModuleId {
        self.ir.module.id()
    }

    /// Returns the deterministic load cost for this module.
    pub fn cost(&self) -> u64 {
        self.cost
    }

    /// Returns the monomorphized function pointer for the given name. If the
    /// function has not been monomorphized, returns [`None`].
    pub fn get_function_ptr(
        &self,
        name: InternedIdentifier,
    ) -> anyhow::Result<Option<FunctionPtr>> {
        Ok(self.get_function_slot(name)?.get().map(|f| f.function))
    }

    /// Returns the function slot for the given name where monomorphized code
    /// may or may not be installed.
    pub fn get_function_slot(
        &self,
        name: InternedIdentifier,
    ) -> anyhow::Result<&OnceLock<FunctionSlot>> {
        self.functions
            .get(&name)
            .ok_or_else(|| anyhow!("Linker error: function not found"))
    }

    /// Returns the polymorphic IR for the function with the given name.
    pub fn get_function_ir(&self, name: InternedIdentifier) -> anyhow::Result<&FunctionIR> {
        let idx = *self
            .function_indices
            .get(&name)
            .ok_or_else(|| anyhow!("Linker error: function not found"))?;
        self.ir
            .functions
            .get(idx)
            .and_then(|slot| slot.as_ref())
            .ok_or_else(|| anyhow!("Linker error: function IR missing"))
    }
}

impl Drop for LoadedModule {
    // SAFETY: A module is only dropped on two paths, both of which exclude
    // live aliases to its lowered function allocations:
    //   1. Maintenance mode clearing module cache. The maintenance guard
    //      guarantees there are no execution guards, so no interpreter is
    //      mid-call and no function pointer alias exists other than in the
    //      cache.
    //   2. When inserting module into cache and losing the race, the loser
    //      is dropped. In this case just-leaked box was never published into
    //      any slot, so it has no aliases by construction.
    //
    // TODO: `FunctionPtr`s in other modules' `CallDirect` ops are only sound
    //   if callers are evicted with direct callees. (or their code is de-optimized).
    fn drop(&mut self) {
        self.functions.retain(|_, cell| {
            if let Some(slot) = cell.take() {
                // SAFETY: see impl-level comment — no aliases at drop time.
                unsafe { slot.function.free_unchecked() };
            }
            false
        });
    }
}
