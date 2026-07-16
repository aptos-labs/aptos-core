// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Implementation of loader to load modules from storage into the long-living
//! cache and per-transaction read-set with deterministic gas charging.
//!
//! Loading modules includes multiple passes:
//!
//! 1. **Charge gas.**
//! Whether the module is in the cache or not, gas is charged deterministically
//! for target module and its mandatory dependencies (i.e., modules which are
//! always preloaded with the target module). After this pass, all modules that
//! were touched are added to the read-set.
//!
//! 2. **Translate (on cache miss only).**
//! For every cache miss, modules are fetched from storage, deserialized,
//! verified and translated into stackless execution IR. Translated modules are
//! then inserted into cache.

use crate::{
    error::{LoaderError, LoaderInvariantViolation, LoaderResult},
    invariant_violation,
    read_set::{ModuleRead, ModuleReadSet, ModuleState},
};
use mono_move_core::{
    interner::{InternedIdentifier, InternedModuleId},
    native::NativeResolver,
    types::{view_name, InternedType, InternedTypeList, EMPTY_TYPE_LIST},
    DescriptorId, FieldTypes, FrameOffset, Function, FunctionPtr, GasMeter, LayoutId,
    LayoutProvider, ModuleId, ModuleProvider, ValueLayout,
};
use mono_move_global_context::{
    ArenaRef, ExecutionGuard, FunctionSlot, LoadedModule, LoadedModuleSlot,
    ModuleMandatoryDependencies, ModuleSlot,
};
use shared_dsa::UnorderedSet;
use specializer::{
    lower::context::{
        try_discover_types_for_lowering_in_function, try_discover_types_for_lowering_in_module,
        try_lower_function, LoweringOutcome, SpecializerContext,
    },
    LoweringResult, ModuleIR,
};
use std::sync::Arc;

/// Describes the lowering policy for converting execution IR to micro-ops.
pub enum LoweringPolicy {
    /// No extra modules loaded. Lowering of any function that needs external
    /// size information is deferred to first call.
    Lazy,
    /// Additionally loads modules that form the transitive closure reachable
    /// from the target's module struct definitions. This makes loading of any
    /// non-generic function possible at load-time.
    ///
    /// ## Example
    ///
    /// ```move
    /// module m0 {
    ///   struct M0 { x: m1::M1, y: u8 }
    ///   fun f(x: &M0): u8 { x.y }
    /// }
    ///
    /// module m1 {
    ///   struct M1 { x: u64, y: u8 }
    ///   struct N1 { x: m2::N2, y: u8 }
    /// }
    ///
    /// module m2 {
    ///   struct N2 { x: u64, y: u8 }
    /// }
    /// ```
    ///
    /// Under eager policy, when loading `m0`, module `m1` is also loaded so
    /// that `f` can be lowered (layout of `M1` needs to be known). Note that
    /// functions in `m1` may not be lowered because it is loaded only to be
    /// able to compute the layout.
    Eager,
}

/// Describes the loading policy for modules.
pub enum LoadingPolicy {
    /// Loads one module at a time. More modules can be loaded based on the
    /// lowering policy.
    Lazy(LoweringPolicy),
    /// Loads all modules in the same package as a single atomic unit. For now,
    /// supports only lazy lowering where functions are lowered only when the
    /// information for lowering is accessible in the package.
    Package,
}

/// Per-transaction code loader: loads code from the cache, charges gas on
/// load, handles cache misses, and records each loaded executable in the
/// transaction's read-set.
pub struct Loader<'guard, 'ctx> {
    guard: &'guard ExecutionGuard<'ctx>,
    module_provider: &'guard dyn ModuleProvider,
    policy: LoadingPolicy,
    natives: &'guard dyn NativeResolver,
}

impl<'guard, 'ctx> Loader<'guard, 'ctx> {
    /// Creates a new loader. The provided [`ModuleProvider`] processes cache
    /// misses: fetch code from storage, deserialize and verify. Policy
    /// dictates how the code is loaded. The [`NativeResolver`] resolves
    /// native call sites during specialization; pass [`NoNatives`] when
    /// no natives are registered.
    pub fn new_with_policy(
        guard: &'guard ExecutionGuard<'ctx>,
        module_provider: &'guard dyn ModuleProvider,
        policy: LoadingPolicy,
        natives: &'guard dyn NativeResolver,
    ) -> Self {
        Self {
            guard,
            module_provider,
            policy,
            natives,
        }
    }

    /// Returns the execution guard this loader is bound to.
    pub fn guard(&self) -> &'guard ExecutionGuard<'ctx> {
        self.guard
    }

    /// Loads and returns the executable corresponding to the given ID,
    /// records it (and any policy-dictated mandatory dependencies) in the
    /// transaction's read-set, and charges gas for the load.
    ///
    /// # Precondition
    ///
    /// The code has not been resolved and added to the read-set yet.
    pub fn load_module(
        &self,
        read_set: &mut ModuleReadSet<'guard>,
        gas_meter: &mut GasMeter,
        id: ArenaRef<'guard, ModuleId>,
    ) -> LoaderResult<&'guard LoadedModule> {
        match &self.policy {
            LoadingPolicy::Lazy(lowering) => {
                use LoweringPolicy::*;
                match lowering {
                    Lazy => self.load_lazy_with_lazy_lowering(read_set, gas_meter, id),
                    Eager => self.load_lazy_with_eager_lowering(read_set, gas_meter, id),
                }
            },
            LoadingPolicy::Package => self.load_package(read_set, gas_meter, id),
        }
    }

    // TODO(cleanup): Revisit the handling of native functions here.
    //
    // Need to make sure:
    // 1. A registered native function impl does not shadow a Move-body function with the same name.
    // 2. A missing native function impl only triggers an error when it's actually being called, not
    //    during load time.
    pub fn load_function(
        &self,
        read_set: &mut ModuleReadSet<'guard>,
        gas_meter: &mut GasMeter,
        module_id: InternedModuleId,
        func_name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> LoaderResult<FunctionPtr> {
        let id = self.guard.arena_ref_for_module_id(module_id);

        let module = match read_set.get(id) {
            Some(ModuleRead::Loaded { module, state }) => match state {
                ModuleState::ReadyForLowering => module,
                ModuleState::Metered => {
                    self.ensure_ready_for_lowering(read_set, gas_meter, id, module)?;
                    module
                },
                ModuleState::Unmetered => {
                    invariant_violation!(ReadSetEntryNotMetered);
                },
            },
            Some(ModuleRead::Pending) => invariant_violation!(ReadSetEntryNotLoaded),
            None => self.load_module(read_set, gas_meter, id)?,
        };

        // Non-generic call.
        if ty_args.is_empty() {
            let slot = module.get_function_slot(func_name).ok_or_else(|| {
                LoaderError::FunctionNotFound {
                    address: *id.address(),
                    module: id.name().to_string(),
                    name: view_name(func_name).to_string(),
                }
            })?;
            if let Some(loaded) = slot.get() {
                self.charge_non_read_set_slots(
                    read_set,
                    gas_meter,
                    &loaded.mandatory_dependencies,
                )?;
                return Ok(loaded.function);
            }
            let (function, function_ms) = self.lower_function_with_ty_args(
                read_set,
                gas_meter,
                module,
                func_name,
                EMPTY_TYPE_LIST,
            )?;
            if let Err(loser) = slot.set(FunctionSlot::new(function, function_ms)) {
                // SAFETY: There are no aliases and this is a unique pointer
                // because it lost the race: safe to free.
                unsafe { loser.function.free_unchecked() };
            }
            let Some(f) = slot.get() else {
                invariant_violation!(FunctionSlotEmptyAfterSet);
            };
            return Ok(f.function);
        }

        // Otherwise this function is generic. Need to lookup in a separate
        // cache. The hot path performs a single hashtable lookup (and a
        // single lock acquisition) and returns. On the rare cache miss we
        // run the lowering pipeline without the lock held, then take the
        // lock a second time to publish the result. Splitting the read and
        // the install keeps the lock window short for the common hit case
        // and avoids holding it across lowering work.
        //
        // TODO(perf): the cache key distinguishes instantiations that
        // differ only in phantom type arguments even though their lowered
        // code is identical. A per-callee mask of the type parameters that
        // actually affect lowering would let the key ignore the rest;
        // same-package callers could also stop forwarding unused ty_args,
        // but upgradable code can't (the caller can't tell which params the
        // callee currently needs).
        if let Some((function, function_ms)) =
            module.get_instantiated_function_ptr(func_name, ty_args)
        {
            self.charge_non_read_set_slots(read_set, gas_meter, &function_ms)?;
            return Ok(function);
        }

        // Cache miss: monomorphize & lower, charging gas.
        let (function, function_ms) =
            self.lower_function_with_ty_args(read_set, gas_meter, module, func_name, ty_args)?;
        Ok(module.set_instantiated_function(func_name, ty_args, function, function_ms))
    }

    /// Runs the lowering pipeline for a single function with the given
    /// substitution table. Returns a fresh `FunctionSlot` containing the
    /// lowered code and its mandatory-dependency set.
    fn lower_function_with_ty_args(
        &self,
        read_set: &mut ModuleReadSet<'guard>,
        gas_meter: &mut GasMeter,
        module: &LoadedModule,
        func_name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> LoaderResult<(Function, Arc<[LoadedModuleSlot]>)> {
        let func_ir = match module.get_function_ir(func_name) {
            Some(Some(ir)) => ir,
            Some(None) => return Err(LoaderError::FunctionIrMissing),
            None => {
                let id = self.guard.arena_ref_for_module_id(module.id());
                return Err(LoaderError::FunctionNotFound {
                    address: *id.address(),
                    module: id.name().to_string(),
                    name: view_name(func_name).to_string(),
                });
            },
        };
        // TODO(metering): the lowering work needs to be charged deterministically.
        let mut loading_ctx = LoweringContext::new(self, read_set);
        let descriptors = try_discover_types_for_lowering_in_function(
            &mut loading_ctx,
            self.guard,
            module.ir(),
            func_ir,
            ty_args,
        )?;

        let parent_ms_ids = module
            .mandatory_dependencies()
            .slots()
            .iter()
            .map(|slot| self.module_slot(slot).id())
            .collect::<UnorderedSet<_>>();
        loading_ctx
            .discovered
            .retain(|slot| !parent_ms_ids.contains(&self.module_slot(slot).id()));
        let function_ms = Arc::<[LoadedModuleSlot]>::from(loading_ctx.discovered);

        self.record_loaded_and_charge_slots(read_set, gas_meter, &function_ms, |_, _| {
            invariant_violation!(UnexpectedReadSetMiss);
        })?;

        let function = match try_lower_function(
            module.ir(),
            func_ir,
            ty_args,
            self.guard,
            self.guard,
            descriptors,
            self.natives,
        )? {
            LoweringOutcome::Built(f) => f,
            // TODO(cleanup): drop this arm — together with the `LoweringOutcome`
            // enum and the corresponding `BuildContextOutcome::Skipped`
            // paths in the specializer — once `try_build_context`
            // handles nominal types and partial concretization. At that
            // point `try_lower_function` is total and can go back to
            // returning `Result<Function>` directly.
            LoweringOutcome::Skipped(reason) => {
                return Err(LoaderError::LoweringSkipped { reason })
            },
        };
        Ok((function, function_ms))
    }
}

//
// Only private APIs below.
// ------------------------

impl<'guard, 'ctx> Loader<'guard, 'ctx> {
    /// Loads only the code corresponding to the specified ID and charges
    /// gas for this code instance.
    fn load_lazy_with_lazy_lowering(
        &self,
        read_set: &mut ModuleReadSet<'guard>,
        gas_meter: &mut GasMeter,
        id: ArenaRef<'guard, ModuleId>,
    ) -> LoaderResult<&'guard LoadedModule> {
        read_set.record_pending_loading(id)?;
        let module = match self.guard.get_module(id) {
            Some(module) => module,
            None => {
                self.build_and_insert_module_ir(id, ModuleMandatoryDependencies::lazy_unset())?
            },
        };

        read_set.record_ready_for_lowering(id, module)?;
        gas_meter.charge(module.cost())?;
        Ok(module)
    }

    /// Loads the code corresponding to the specified ID and all other
    /// modules in the same package. Gas is charged for the whole package
    /// whether it was cache miss or hit.
    fn load_package(
        &self,
        read_set: &mut ModuleReadSet<'guard>,
        gas_meter: &mut GasMeter,
        id: ArenaRef<'guard, ModuleId>,
    ) -> LoaderResult<&'guard LoadedModule> {
        let package = match self.guard.get_module(id) {
            Some(module) => module.mandatory_dependencies().clone(),
            None => self.build_mandatory_dependencies_for_id(id)?,
        };

        // If cache hit, we need to go over slots, record them in the read-set,
        // and charge gas. If cache miss, we do the same but also fetch modules
        // from storage on read-set cache miss and insert them into slots and
        // read-set.
        self.record_loaded_and_charge_slots(
            read_set,
            gas_meter,
            package.slots(),
            |read_set, slot| {
                let id = self.guard.arena_ref_for_module_id(slot.id());
                read_set.record_pending_loading(id)?;
                let module = match slot.get(self.guard) {
                    Some(module) => module,
                    None => self.build_and_insert_module_ir(id, package.clone())?,
                };
                read_set.record_ready_for_lowering(id, module)?;
                Ok(module)
            },
        )?;

        // Promote any package member that was already in the read-set as
        // metered (e.g., a layout-only side-load earlier in this transaction).
        for slot in package.slots() {
            let slot_id = self
                .guard
                .arena_ref_for_module_id(self.module_slot(slot).id());
            if matches!(
                read_set.get(slot_id),
                Some(ModuleRead::Loaded {
                    state: ModuleState::Metered,
                    ..
                })
            ) {
                read_set.mark_ready_for_lowering(slot_id)?;
            }
        }

        if let Some(ModuleRead::Loaded { module, state }) = read_set.get(id) {
            if !matches!(state, ModuleState::ReadyForLowering) {
                invariant_violation!(TargetModuleNotReady);
            }
            Ok(module)
        } else {
            invariant_violation!(TargetModuleNotLoaded)
        }
    }

    /// Builds mandatory module dependencies to add to a module that have just
    /// been loaded.
    fn build_mandatory_dependencies_for_id(
        &self,
        id: ArenaRef<'guard, ModuleId>,
    ) -> LoaderResult<ModuleMandatoryDependencies> {
        match &self.policy {
            LoadingPolicy::Lazy(_) => Ok(ModuleMandatoryDependencies::lazy_unset()),
            LoadingPolicy::Package => {
                let module_names = self
                    .module_provider
                    .get_same_package_modules(id.address(), id.name())
                    .map_err(LoaderError::ModuleProvider)?;
                let package_slots = module_names
                    .into_iter()
                    .map(|module_name| {
                        let module_id = self
                            .guard
                            .intern_address_name(id.address(), module_name.as_ident_str());
                        self.guard.get_or_create_module_slot(module_id)
                    })
                    .collect::<Vec<_>>();
                Ok(ModuleMandatoryDependencies::package(package_slots))
            },
        }
    }

    /// Loads the code corresponding to the specified ID and all other
    /// modules that are needed for lowering of all functions in this
    /// module. Gas is charged for the whole set of these modules.
    fn load_lazy_with_eager_lowering(
        &self,
        read_set: &mut ModuleReadSet<'guard>,
        gas_meter: &mut GasMeter,
        id: ArenaRef<'guard, ModuleId>,
    ) -> LoaderResult<&'guard LoadedModule> {
        let module = match self.guard.get_module(id) {
            None => {
                read_set.record_pending_loading(id)?;
                let module =
                    self.build_and_insert_module_ir(id, ModuleMandatoryDependencies::lazy_unset())?;
                read_set.record_unmetered(id, module)?;
                module
            },
            Some(module) => {
                let Some(deps) = module.mandatory_dependencies().as_lazy() else {
                    invariant_violation!(MandatoryDepsNotLazy);
                };
                if deps.get().is_some() {
                    // Mandatory set is already cached - only need to charge gas.
                    self.charge_mandatory_set_for_eager_lowering(read_set, gas_meter, id, module)?;
                    return Ok(module);
                }

                // Dependencies not yet set: either a concurrent eager load
                // hasn't filled them, or this module was inserted earlier as
                // a layout side-effect. Record self as loaded-but-unmetered
                // and fall through to the walker.
                read_set.record_pending_loading(id)?;
                read_set.record_unmetered(id, module)?;
                module
            },
        };

        self.compute_mandatory_set_for_eager_lowering(read_set, gas_meter, id, module)?;
        Ok(module)
    }

    /// For metered module in the read-set, ensures the module is in ready for
    /// lowering state.
    fn ensure_ready_for_lowering(
        &self,
        read_set: &mut ModuleReadSet<'guard>,
        gas_meter: &mut GasMeter,
        id: ArenaRef<'guard, ModuleId>,
        module: &'guard LoadedModule,
    ) -> LoaderResult<()> {
        match &self.policy {
            LoadingPolicy::Lazy(LoweringPolicy::Lazy) => {
                // Nothing extra to charge, safe to mark ready for lowering.
                read_set.mark_ready_for_lowering(id)?;
            },
            LoadingPolicy::Lazy(LoweringPolicy::Eager) => {
                let Some(deps) = module.mandatory_dependencies().as_lazy() else {
                    invariant_violation!(MandatoryDepsNotLazy);
                };
                if deps.get().is_some() {
                    self.charge_mandatory_set_for_eager_lowering(read_set, gas_meter, id, module)?;
                } else {
                    self.compute_mandatory_set_for_eager_lowering(read_set, gas_meter, id, module)?;
                }
            },
            LoadingPolicy::Package => {
                // The metered state can arise from a layout-only side-load
                // earlier in the transaction. Load the full package now to
                // charge any missing siblings and promote them to ready.
                self.load_package(read_set, gas_meter, id)?;
            },
        }
        Ok(())
    }

    /// Charges gas for every module in the already-cached mandatory dependency
    /// set that is not yet metered, and marks the target ready for lowering.
    fn charge_mandatory_set_for_eager_lowering(
        &self,
        read_set: &mut ModuleReadSet<'guard>,
        gas_meter: &mut GasMeter,
        id: ArenaRef<'guard, ModuleId>,
        module: &'guard LoadedModule,
    ) -> LoaderResult<()> {
        let Some(deps) = module.mandatory_dependencies().as_lazy() else {
            invariant_violation!(MandatoryDepsNotLazy);
        };
        let Some(slots) = deps.get() else {
            invariant_violation!(MandatoryDepsNotSet);
        };
        self.charge_non_read_set_slots(read_set, gas_meter, slots)?;
        read_set.mark_ready_for_lowering(id)?;
        Ok(())
    }

    /// Walks the target module's lowering type closure to compute its mandatory
    /// set, installs  the resulting set for the module. Then charges gas for
    /// every member in the set not yet metered, and marks the target ready for
    /// lowering.
    ///
    /// # Precondition
    ///
    /// The target module is loaded to the read-set and is either metered or
    /// unmetered.
    fn compute_mandatory_set_for_eager_lowering(
        &self,
        read_set: &mut ModuleReadSet<'guard>,
        gas_meter: &mut GasMeter,
        id: ArenaRef<'guard, ModuleId>,
        module: &'guard LoadedModule,
    ) -> LoaderResult<()> {
        let mut walker = LoweringContext::new(self, read_set);
        let self_slot = self.guard.get_or_create_module_slot(id);
        walker.discovered_seen.insert(module.id());
        walker.discovered.push(self_slot);

        // Per-function lowering re-walks types and rebuilds its own
        // descriptor map; only the side-effecting publish-to-guard
        // matters here.
        let _ = try_discover_types_for_lowering_in_module(&mut walker, self.guard, module.ir())?;

        // Set the mandatory set for the module. Because of concurrency, it is
        // possible that other thread sets it at before, so we need to reload
        // it.
        let Some(deps) = module.mandatory_dependencies().as_lazy() else {
            invariant_violation!(MandatoryDepsNotLazy);
        };
        let _ = deps.set(walker.discovered.into());
        let Some(ms) = deps.get() else {
            invariant_violation!(MandatoryDepsNotSet);
        };

        // For all modules in mandatory set, charge gas. This charging also
        // includes self. Once done, we need to mark it as ready for lowering.
        self.record_loaded_and_charge_slots(read_set, gas_meter, ms, |_, _| {
            invariant_violation!(UnexpectedReadSetMiss)
        })?;
        read_set.mark_ready_for_lowering(id)?;
        Ok(())
    }

    /// Fetches, deserializes, and verifies the module from storage, returning
    /// it alongside its deterministic cost (byte length).
    fn get_verified_module_from_storage(
        &self,
        id: ArenaRef<'guard, ModuleId>,
    ) -> LoaderResult<(ModuleIR, u64)> {
        let bytes = self
            .module_provider
            .get_module_bytes(id.address(), id.name())
            .map_err(LoaderError::ModuleProvider)?
            .ok_or_else(|| LoaderError::ModuleNotFound {
                address: *id.address(),
                name: id.name().to_string(),
            })?;
        // TODO(metering): placeholder cost model — byte length of the module. Replace
        // with a proper cost function (bucketed by size, verifier cost, etc.).
        let cost = bytes.len() as u64;
        let compiled_module = self
            .module_provider
            .deserialize_module(&bytes)
            .map_err(LoaderError::Deserialization)?;
        self.module_provider
            .verify_module(&compiled_module)
            .map_err(LoaderError::Verification)?;
        // TODO(cleanup):
        //   This can run verification twice because destack runs it and we verified before.
        //   Destack should take a hook so we can add more things to verify.
        let module_ir = specializer::destack(compiled_module, self.guard)?;
        Ok((module_ir, cost))
    }

    /// Called if module does not exist in the cache.
    ///
    /// Module is fetched from storage, deserialized, verified, translated to
    /// execution IR and inserted into the module cache. The reference to the
    /// inserted module is returned.
    ///
    /// Note: There can be multiple concurrent insertions into the cache. The
    /// cache ensures that a single insertion wins, returning the "canonical"
    /// module reference.
    fn build_and_insert_module_ir(
        &self,
        id: ArenaRef<'guard, ModuleId>,
        deps: ModuleMandatoryDependencies,
    ) -> LoaderResult<&'guard LoadedModule> {
        let (module_ir, cost) = self.get_verified_module_from_storage(id)?;
        self.guard
            .insert_module(LoadedModule::new(module_ir, cost, deps))
            .map_err(LoaderError::GlobalContext)
    }

    /// Records all modules in the slots in the read-set and charges its cost
    /// as a sum.
    fn record_loaded_and_charge_slots<F>(
        &self,
        read_set: &mut ModuleReadSet<'guard>,
        gas_meter: &mut GasMeter,
        slots: &[LoadedModuleSlot],
        mut on_read_set_miss: F,
    ) -> LoaderResult<()>
    where
        F: FnMut(&mut ModuleReadSet<'guard>, &ModuleSlot) -> LoaderResult<&'guard LoadedModule>,
    {
        let mut loading_cost = 0u64;
        for slot in slots.iter().map(|s| self.module_slot(s)) {
            let id = self.guard.arena_ref_for_module_id(slot.id());
            match read_set.get(id) {
                Some(ModuleRead::Loaded { module, state }) => match state {
                    ModuleState::ReadyForLowering | ModuleState::Metered => continue,
                    ModuleState::Unmetered => {
                        loading_cost = loading_cost.saturating_add(module.cost());
                        read_set.mark_metered(id)?;
                    },
                },
                Some(ModuleRead::Pending) => invariant_violation!(ReadSetEntryNotLoaded),
                None => {
                    let module = on_read_set_miss(read_set, slot)?;
                    loading_cost = loading_cost.saturating_add(module.cost());
                },
            }
        }
        gas_meter.charge(loading_cost)?;
        Ok(())
    }

    /// Charges gas for every module in the specified slots that is not yet in
    /// the read-set.
    fn charge_non_read_set_slots(
        &self,
        read_set: &mut ModuleReadSet<'guard>,
        gas_meter: &mut GasMeter,
        slots: &[LoadedModuleSlot],
    ) -> LoaderResult<()> {
        self.record_loaded_and_charge_slots(read_set, gas_meter, slots, |read_set, slot| {
            let id = self.guard.arena_ref_for_module_id(slot.id());
            read_set.record_pending_loading(id)?;
            let module = match slot.get(self.guard) {
                Some(module) => module,
                None => {
                    self.build_and_insert_module_ir(id, ModuleMandatoryDependencies::lazy_unset())?
                },
            };
            read_set.record_metered(id, module)?;
            Ok(module)
        })
    }

    fn module_slot(&self, slot: &LoadedModuleSlot) -> &'guard ModuleSlot {
        // SAFETY: Loader owns guard, which means that the slot pointer stays
        // throughout loader's lifetime.
        unsafe { slot.as_ref_unchecked() }
    }
}

/// Records modules visited during lowering requirements calculation in the
/// read-set.
struct LoweringContext<'a, 'guard, 'ctx> {
    loader: &'a Loader<'guard, 'ctx>,
    read_set: &'a mut ModuleReadSet<'guard>,
    /// All modules needed for lowering of this function, ordered based on the
    /// specializer DFS type traversal.
    discovered: Vec<LoadedModuleSlot>,
    discovered_seen: UnorderedSet<InternedModuleId>,
}

impl<'a, 'guard, 'ctx> LoweringContext<'a, 'guard, 'ctx> {
    fn new(loader: &'a Loader<'guard, 'ctx>, read_set: &'a mut ModuleReadSet<'guard>) -> Self {
        Self {
            loader,
            read_set,
            discovered: vec![],
            discovered_seen: UnorderedSet::new(),
        }
    }
}

impl LayoutProvider for LoweringContext<'_, '_, '_> {
    fn layout(&self, id: LayoutId) -> Option<&ValueLayout> {
        self.loader.guard.layout(id)
    }

    fn layout_id(&self, ty: InternedType) -> Option<LayoutId> {
        self.loader.guard.layout_id(ty)
    }
}

impl SpecializerContext for LoweringContext<'_, '_, '_> {
    fn get_fields(
        &mut self,
        module_id: &InternedModuleId,
        nominal_name: &InternedIdentifier,
    ) -> LoweringResult<Option<FieldTypes>> {
        let id = self.loader.guard.arena_ref_for_module_id(*module_id);

        // Every module needs to be in the read-set.
        let module = match self.read_set.get(id) {
            Some(ModuleRead::Loaded { module, .. }) => module,
            Some(ModuleRead::Pending) => {
                return Err(LoaderError::InvariantViolation(
                    LoaderInvariantViolation::ReadSetEntryNotLoaded,
                )
                .into());
            },
            None => {
                self.read_set.record_pending_loading(id)?;
                let module = match self.loader.guard.get_module(id) {
                    Some(module) => module,
                    None => {
                        let deps = self.loader.build_mandatory_dependencies_for_id(id)?;
                        self.loader.build_and_insert_module_ir(id, deps)?
                    },
                };
                self.read_set.record_unmetered(id, module)?;
                module
            },
        };

        // Accumulate visited module slots so that we can construct mandatory set
        // for the root module later.
        if self.discovered_seen.insert(*module_id) {
            let slot = self.loader.guard.get_or_create_module_slot(id);
            self.discovered.push(slot);
        }

        Ok(module
            .ir()
            .module
            .interned_field_types(*nominal_name)
            .cloned())
    }

    fn publish_vec_descriptor(
        &self,
        elem_ty: InternedType,
        elem_size: u32,
        elem_ptr_offsets: &[FrameOffset],
    ) -> DescriptorId {
        self.loader
            .guard
            .publish_vec_descriptor(elem_ty, elem_size, elem_ptr_offsets)
    }

    fn vec_descriptor_for(&self, elem_ty: InternedType) -> Option<DescriptorId> {
        self.loader.guard.vec_descriptor_for(elem_ty)
    }

    fn publish_enum_descriptor(
        &self,
        enum_ty: InternedType,
        size: u32,
        variant_pointer_offsets: Vec<Vec<u32>>,
    ) -> DescriptorId {
        self.loader
            .guard
            .publish_enum_descriptor(enum_ty, size, variant_pointer_offsets)
    }

    fn publish_captured_data_descriptor(
        &self,
        values_size: u32,
        pointer_offsets: &[FrameOffset],
    ) -> DescriptorId {
        self.loader
            .guard
            .publish_captured_data_descriptor(values_size, pointer_offsets)
    }

    fn publish_layout(&self, ty: InternedType, layout: ValueLayout) -> LayoutId {
        self.loader.guard.publish_layout(ty, layout)
    }

    fn publish_variant_layouts(
        &self,
        enum_ty: InternedType,
        variants: Vec<ValueLayout>,
    ) -> Box<[LayoutId]> {
        self.loader.guard.publish_variant_layouts(enum_ty, variants)
    }

    fn publish_struct_descriptor(
        &self,
        struct_ty: InternedType,
        size: u32,
        ptr_offsets: &[FrameOffset],
    ) -> DescriptorId {
        self.loader
            .guard
            .publish_struct_descriptor(struct_ty, size, ptr_offsets)
    }
}
