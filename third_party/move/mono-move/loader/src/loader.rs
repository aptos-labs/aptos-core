// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Implementation of loader to load modules from storage into the long-living
//! cache with deterministic gas charging.
//!
//! Loading modules includes multiple passes:
//!
//! 1. **Charge gas.**
//! Whether the module is in the cache or not, gas is charged deterministically
//! for target module and its mandatory dependencies (i.e., modules which are
//! always preloaded with the target module). Each module is charged at most
//! once per transaction: the execution guard records the ones already charged.
//!
//! 2. **Translate (on cache miss only).**
//! For every cache miss, modules are fetched from storage, deserialized,
//! verified and translated into stackless execution IR. Translated modules are
//! then inserted into cache.

use crate::{error::LoaderError, invariant_violation};
use mono_move_core::{
    abilities::{AbilityCalculator, AbilityError},
    interner::{
        script_module_id, view_module_id, InternedFunctionRef, InternedIdentifier,
        InternedModuleId, SCRIPT_MAIN,
    },
    native::{FunctionResolutionError, NativeResolver},
    types::{
        infer_function_type_args, view_name, view_type, view_type_list, FunctionTypeMismatch,
        InternedType, InternedTypeList, Type, EMPTY_TYPE_LIST,
    },
    verify_function, DescriptorId, ErrorLocation, ExecutionErrorKind, FrameOffset,
    FrameworkSymbols, Function, FunctionPtr, GasMeter, Interner, LayoutId, LayoutProvider,
    ModuleId, ModuleProvider, NominalFields, PreparedModule, VMInternalError, VMResult,
    ValueLayout,
};
use mono_move_global_context::{
    ArenaRef, ExecutionGuard, FunctionIrLookup, FunctionSlot, LoadedModule, ModuleIdx, ScriptHash,
    SCRIPT_MODULE_IDX,
};
use move_binary_format::{
    access::{ModuleAccess, ScriptAccess},
    errors::VMError,
    file_format::{CompiledScript, Visibility},
    module_script_conversion::script_into_module,
};
use move_core_types::{ability::AbilitySet, identifier::IdentStr};
use shared_dsa::{UnorderedMap, UnorderedSet};
use specializer::{
    lower::context::{
        publish_resource_type, try_discover_types_for_lowering_in_function,
        try_discover_types_for_lowering_in_module, try_lower_function, LoweringOutcome,
        SpecializerContext,
    },
    ModuleIR,
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
/// load, handles cache misses, and records each module it charged on the
/// execution guard.
pub struct Loader<'guard, 'ctx> {
    guard: &'guard ExecutionGuard<'ctx>,
    module_provider: &'guard dyn ModuleProvider,
    policy: LoadingPolicy,
    natives: &'guard dyn NativeResolver,
}

/// Preserves the verifier's error details, including its location.
fn script_verification_failed(error: VMError) -> VMInternalError {
    VMInternalError::new(LoaderError::ScriptVerificationFailed { error })
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

    /// The module with the specified ID, which this transaction has already
    /// charged for and so must be loaded.
    pub fn loaded_module(&self, module_id: InternedModuleId) -> VMResult<&'guard LoadedModule> {
        self.loaded_module_at(self.module_idx(module_id)?)
    }

    /// Loads and returns the executable corresponding to the given ID, records
    /// it (and any policy-dictated mandatory dependencies) as charged on the
    /// guard, and charges gas for the load. A module this transaction has
    /// already charged for is returned without charging again.
    pub fn load_module(
        &self,
        gas_meter: &mut GasMeter,
        id: ArenaRef<'guard, ModuleId>,
    ) -> VMResult<&'guard LoadedModule> {
        match &self.policy {
            LoadingPolicy::Lazy(lowering) => {
                use LoweringPolicy::*;
                match lowering {
                    Lazy => self.load_lazy_with_lazy_lowering(gas_meter, id),
                    Eager => self.load_lazy_with_eager_lowering(gas_meter, id),
                }
            },
            LoadingPolicy::Package => self.load_package(gas_meter, id),
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
        gas_meter: &mut GasMeter,
        module_id: InternedModuleId,
        func_name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> VMResult<FunctionPtr> {
        let idx = self.module_idx(module_id)?;
        self.load_function_at(gas_meter, idx, func_name, ty_args)
    }

    /// As [`Loader::load_function`], but names the callee's module by its table
    /// index. Lowered call sites carry the index, so a call resolves without
    /// hashing the module ID.
    pub fn load_function_at(
        &self,
        gas_meter: &mut GasMeter,
        idx: ModuleIdx,
        func_name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> VMResult<FunctionPtr> {
        let module = if self.guard.is_charged(idx) {
            let module = self.loaded_module_at(idx)?;
            // A script is loaded whole by `load_script` and has no module in
            // storage to walk a lowering set from.
            if idx != SCRIPT_MODULE_IDX {
                self.ensure_ready_for_lowering(gas_meter, module)?;
            }
            module
        } else {
            self.load_module(gas_meter, self.module_id_at(idx)?)?
        };

        // Non-generic call.
        if ty_args.is_empty() {
            let Some(slot) = module.get_function_slot(func_name) else {
                // No lowered-code slot: either a native (present by name, no IR)
                // or genuinely absent: report which.
                let id = self.module_id_at(idx)?;
                return Err(VMInternalError::new(
                    match module.get_function_ir(func_name) {
                        // TODO(completeness): a closure over a native is packable
                        // but not callable, where the legacy VM calls it. A call
                        // site has no `NativeABI`, so the fix is to synthesize one
                        // per instantiation at load time.
                        FunctionIrLookup::Native => LoaderError::NativeFunctionNotLoadable {
                            address: *id.address(),
                            module: id.name().to_string(),
                            name: view_name(func_name).to_string(),
                        },
                        FunctionIrLookup::Ir(_) | FunctionIrLookup::NotDefined => {
                            LoaderError::FunctionNotFound {
                                address: *id.address(),
                                module: id.name().to_string(),
                                name: view_name(func_name).to_string(),
                            }
                        },
                    },
                ));
            };
            if let Some(loaded) = slot.get() {
                self.charge_and_load(gas_meter, &loaded.mandatory_dependencies, None)?;
                return Ok(loaded.function);
            }
            let (function, function_ms) =
                self.lower_function_with_ty_args(gas_meter, module, func_name, EMPTY_TYPE_LIST)?;
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
            self.charge_and_load(gas_meter, &function_ms, None)?;
            return Ok(function);
        }

        // Cache miss: monomorphize & lower, charging gas.
        let (function, function_ms) =
            self.lower_function_with_ty_args(gas_meter, module, func_name, ty_args)?;
        Ok(module.set_instantiated_function(func_name, ty_args, function, function_ms))
    }

    /// Publishes the layout and GC descriptor of the resource type `ty`, so a
    /// read of it can be materialized outside lowered code. Loads and charges
    /// for the modules its definition pulls in.
    pub fn publish_resource_type(
        &self,
        gas_meter: &mut GasMeter,
        ty: InternedType,
    ) -> VMResult<()> {
        let mut ctx = LoweringContext::new(self);
        let published = publish_resource_type(&mut ctx, self.guard, ty)?;
        self.charge_and_load(gas_meter, &ctx.discovered, None)?;
        if !published {
            return Err(VMInternalError::new(
                LoaderError::ResourceLayoutNotDerivable,
            ));
        }
        Ok(())
    }

    /// Resolves `module_id::func_name` and checks its type matches the
    /// expected type. If the type turns out to be different, resolution fails.
    pub fn resolve_function(
        &self,
        gas_meter: &mut GasMeter,
        module_id: InternedModuleId,
        func_name: &IdentStr,
        expected_ty: InternedType,
    ) -> VMResult<Result<InternedFunctionRef, FunctionResolutionError>> {
        use FunctionResolutionError::*;

        // Charging happens before the load, so a module charged for but not
        // loaded is one whose load failed earlier in this transaction.
        // Resolution is the one caller that survives a failed load, so it has
        // to fail the same way again instead of tripping over the leftover.
        let idx = self.module_idx(module_id)?;
        if self.guard.is_charged(idx) && self.guard.module_at(idx).is_none() {
            return Ok(Err(FunctionNotFound));
        }

        let module = match self.get_or_load_module(gas_meter, module_id) {
            Ok(module) => module,
            Err(err) if err.kind() == ExecutionErrorKind::LinkingError => {
                return Ok(Err(FunctionNotFound))
            },
            Err(err) => return Err(err),
        };
        // TODO(security): a caller-supplied name that names nothing still ends
        // up in the process-global interner. Same class as the `TODO(metering)`
        // on `ExecutionGuard::intern_identifier_internal`.
        let func_name = self
            .guard
            .intern_identifier(func_name)
            .into_global_arena_ptr();
        let Some(def_idx) = module.function_def_idx(func_name) else {
            return Ok(Err(FunctionNotFound));
        };

        let prepared = &module.ir().module;
        let def = prepared.function_def_at(def_idx);
        if !matches!(def.visibility, Visibility::Public) {
            return Ok(Err(FunctionNotAccessible));
        }

        let Type::Function { abilities, .. } = view_type(expected_ty) else {
            return Ok(Err(FunctionIncompatibleType));
        };
        // A resolved function is public and captures nothing, so the closure
        // built from it has exactly the abilities of a public function value.
        // Anything stronger has no inhabitant.
        if !abilities.is_subset(AbilitySet::PUBLIC_FUNCTIONS) {
            return Ok(Err(FunctionIncompatibleType));
        }

        let constraints = &prepared.function_handle_at(def.function).type_parameters;
        let declared = prepared.function_signature_at(def.function);
        let ty_args = match infer_function_type_args(declared, expected_ty, constraints.len()) {
            Ok(ty_args) => ty_args,
            Err(FunctionTypeMismatch::NotAFunction | FunctionTypeMismatch::Incompatible) => {
                return Ok(Err(FunctionIncompatibleType))
            },
            Err(FunctionTypeMismatch::NotInstantiated) => return Ok(Err(FunctionNotInstantiated)),
        };

        if !self.ty_args_satisfy_constraints(gas_meter, constraints, &ty_args)? {
            return Ok(Err(FunctionIncompatibleType));
        }
        let ty_args = self.guard.type_list_of(&ty_args);
        Ok(Ok(self
            .guard
            .function_ref_of(module_id, func_name, ty_args)))
    }

    /// Whether every type argument satisfies the constraint declared for it.
    ///
    /// A [`Type::Nominal`] carries no abilities, so each module an argument
    /// names has to be loaded to reach the declaration that does.
    fn ty_args_satisfy_constraints(
        &self,
        gas_meter: &mut GasMeter,
        constraints: &[AbilitySet],
        ty_args: &[InternedType],
    ) -> VMResult<bool> {
        if constraints.iter().all(|c| *c == AbilitySet::EMPTY) {
            return Ok(true);
        }

        let mut nominal_modules = vec![];
        for &ty in ty_args {
            collect_nominal_modules(ty, &mut nominal_modules);
        }
        let mut modules = UnorderedMap::with_capacity(nominal_modules.len());
        for nominal_module in nominal_modules {
            let module = self.get_or_load_module(gas_meter, nominal_module)?;
            modules.insert(nominal_module, &module.ir().module);
        }

        let lookup = |module_id, name| {
            modules
                .get(&module_id)
                .and_then(|module: &&PreparedModule| module.nominal_handle(module_id, name))
                .ok_or(AbilityError::UnknownNominal)
        };
        let mut calculator = AbilityCalculator::new(lookup, &[]);
        for (constraint, &ty) in constraints.iter().zip(ty_args) {
            if !constraint.is_subset(calculator.abilities_of(ty)?) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Loads a script from its bytes and returns its `main` instantiated with
    /// `ty_args`. A script is loaded as a module holding that one function,
    /// under a module ID all scripts share, and cached by the hash of its
    /// bytes. Its cost and its dependencies' loads are charged on every call.
    /// On a cache miss, the module provider's configs govern deserialization
    /// and verification.
    ///
    /// The guard's script slot points at the script loaded last, so a caller
    /// that runs several scripts under one guard sees the one it is running.
    pub fn load_script(
        &self,
        gas_meter: &mut GasMeter,
        script_code: &[u8],
        ty_args: InternedTypeList,
    ) -> VMResult<FunctionPtr> {
        let module_id = script_module_id(self.guard);
        // Recorded before the load, so that a script that fails to load is
        // still a read.
        self.guard.mark_charged(SCRIPT_MODULE_IDX);

        let hash = ScriptHash::of(script_code);
        let module = match self.guard.get_script(&hash) {
            Some(module) => {
                // A miss loads the dependencies to link the script. A hit loads
                // them too, so that gas and the recorded reads do not depend on
                // cache warmth.
                let script = &module.ir().module;
                for &dependency in script.module_ids() {
                    if dependency != script.id() {
                        self.get_or_load_module(gas_meter, dependency)?;
                    }
                }
                module
            },
            // TODO(perf): evaluate whether waiting for a concurrent insertion
            // beats every thread verifying the script itself.
            None => self.build_and_insert_script(gas_meter, hash, script_code)?,
        };
        self.guard.set_script(module);
        gas_meter.charge(module.cost())?;

        let main = view_module_id(module_id).name();
        self.load_function(gas_meter, module_id, main, ty_args)
    }

    /// Deserializes, verifies, and links `script_code` against its
    /// dependencies, then caches it as a module.
    fn build_and_insert_script(
        &self,
        gas_meter: &mut GasMeter,
        hash: ScriptHash,
        script_code: &[u8],
    ) -> VMResult<&'guard LoadedModule> {
        let script = CompiledScript::deserialize_with_config(
            script_code,
            self.module_provider.deserializer_config(),
        )
        .map_err(|err| {
            VMInternalError::new(LoaderError::ScriptDeserializationFailed {
                message: err.to_string(),
            })
            .at(ErrorLocation::Script)
        })?;
        let verifier_config = self.module_provider.verifier_config();
        move_bytecode_verifier::verify_script_with_config(verifier_config, &script)
            .map_err(script_verification_failed)?;
        let dependencies = script
            .immediate_dependencies_iter()
            .map(|(address, name)| {
                let module_id = self.guard.module_id_of(address, name);
                self.get_or_load_module(gas_meter, module_id)
            })
            .collect::<VMResult<Vec<_>>>()?;
        move_bytecode_verifier::dependencies::verify_script(
            verifier_config,
            &script,
            dependencies
                .iter()
                .map(|dependency| &*dependency.ir().module),
        )
        .map_err(script_verification_failed)?;

        // TODO(metering): placeholder cost model, as for modules.
        let cost = script_code.len() as u64;
        let module_ir =
            specializer::destack(script_into_module(script, SCRIPT_MAIN.as_str()), self.guard)?;
        let module = LoadedModule::new(module_ir, cost, None, self.guard)
            .map_err(|e| VMInternalError::new(LoaderError::GlobalContext(e)))?;
        Ok(self.guard.insert_script(hash, module))
    }

    /// Returns the module, loading and charging for it first if this
    /// transaction has not yet.
    pub fn get_or_load_module(
        &self,
        gas_meter: &mut GasMeter,
        module_id: InternedModuleId,
    ) -> VMResult<&'guard LoadedModule> {
        let idx = self.module_idx(module_id)?;
        if self.guard.is_charged(idx) {
            return self.loaded_module_at(idx);
        }
        self.load_module(gas_meter, self.guard.arena_ref_for_module_id(module_id))
    }

    /// Runs the lowering pipeline for a single function with the given
    /// substitution table. Returns a fresh `FunctionSlot` containing the
    /// lowered code and its mandatory-dependency set.
    fn lower_function_with_ty_args(
        &self,
        gas_meter: &mut GasMeter,
        module: &LoadedModule,
        func_name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> VMResult<(Function, Arc<[InternedModuleId]>)> {
        let func_ir = match module.get_function_ir(func_name) {
            FunctionIrLookup::Ir(ir) => ir,
            FunctionIrLookup::Native => {
                let id = self.guard.arena_ref_for_module_id(module.id());
                return Err(VMInternalError::new(
                    LoaderError::NativeFunctionNotLoadable {
                        address: *id.address(),
                        module: id.name().to_string(),
                        name: view_name(func_name).to_string(),
                    },
                ));
            },
            FunctionIrLookup::NotDefined => {
                let id = self.guard.arena_ref_for_module_id(module.id());
                return Err(VMInternalError::new(LoaderError::FunctionNotFound {
                    address: *id.address(),
                    module: id.name().to_string(),
                    name: view_name(func_name).to_string(),
                }));
            },
        };
        // TODO(metering): the lowering work, including micro-op verification
        // below, needs to be charged deterministically.
        let mut loading_ctx = LoweringContext::new(self);
        let descriptors = try_discover_types_for_lowering_in_function(
            &mut loading_ctx,
            self.guard,
            module.ir(),
            func_ir,
            ty_args,
        )?;

        // Every filtered module is already loaded: lowering runs only once the
        // parent is ready, which charges all of MS(parent).
        let parent_ms = module
            .mandatory_dependencies()
            .iter()
            .copied()
            .collect::<UnorderedSet<_>>();
        loading_ctx.discovered.retain(|id| !parent_ms.contains(id));
        let function_ms = Arc::<[InternedModuleId]>::from(loading_ctx.discovered);

        self.charge_and_load(gas_meter, &function_ms, None)?;

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
                return Err(VMInternalError::new(LoaderError::LoweringSkipped {
                    reason,
                }))
            },
        };
        // Verify once per lowering, before the function is leaked into a
        // cache: a rejected function is dropped here and never executed.
        let errors = verify_function(&function, self.guard);
        if !errors.is_empty() {
            invariant_violation!(MicroOpVerificationFailed { errors });
        }
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
        gas_meter: &mut GasMeter,
        id: ArenaRef<'guard, ModuleId>,
    ) -> VMResult<&'guard LoadedModule> {
        let module_id = id.into_global_arena_ptr();
        self.charge_and_load(gas_meter, &[module_id], None)?;
        self.loaded_module(module_id)
    }

    /// Loads the code corresponding to the specified ID and all other
    /// modules in the same package. Gas is charged for the whole package
    /// whether it was cache miss or hit.
    fn load_package(
        &self,
        gas_meter: &mut GasMeter,
        id: ArenaRef<'guard, ModuleId>,
    ) -> VMResult<&'guard LoadedModule> {
        // An empty slice means a package with no members, so a module whose
        // set is not computed yet has to be told apart from one that is.
        let package = match self.guard.get_module(id) {
            Some(module) => match module.mandatory_dependencies_if_known() {
                Some(package) => package.clone(),
                None => Arc::from(module.set_mandatory_dependencies(self.package_ids(id)?)),
            },
            None => self.package_ids(id)?,
        };

        // The target is a member of its own package, so this charges for it
        // too. Members already charged earlier in this transaction, by a
        // layout-only side-load for instance, are charged once overall.
        self.charge_and_load(gas_meter, &package, Some(&package))?;

        let idx = self.module_idx(id.into_global_arena_ptr())?;
        match self.guard.module_at(idx) {
            Some(module) => Ok(module),
            None => invariant_violation!(TargetModuleNotLoaded),
        }
    }

    /// Builds mandatory module dependencies to add to a module that have just
    /// been loaded.
    fn build_mandatory_dependencies_for_id(
        &self,
        id: ArenaRef<'guard, ModuleId>,
    ) -> VMResult<Option<Arc<[InternedModuleId]>>> {
        match &self.policy {
            LoadingPolicy::Lazy(_) => Ok(None),
            LoadingPolicy::Package => Ok(Some(self.package_ids(id)?)),
        }
    }

    /// Interned IDs of every module in the same package as `id`, itself
    /// included.
    fn package_ids(&self, id: ArenaRef<'guard, ModuleId>) -> VMResult<Arc<[InternedModuleId]>> {
        let module_names = self
            .module_provider
            .get_same_package_modules(id.address(), id.name())?;
        Ok(module_names
            .into_iter()
            .map(|module_name| {
                self.guard
                    .intern_address_name(id.address(), module_name.as_ident_str())
                    .into_global_arena_ptr()
            })
            .collect::<Arc<[_]>>())
    }

    /// Loads the code corresponding to the specified ID and all other
    /// modules that are needed for lowering of all functions in this
    /// module. Gas is charged for the whole set of these modules.
    fn load_lazy_with_eager_lowering(
        &self,
        gas_meter: &mut GasMeter,
        id: ArenaRef<'guard, ModuleId>,
    ) -> VMResult<&'guard LoadedModule> {
        let idx = self.module_idx(id.into_global_arena_ptr())?;
        let module = self.get_or_build(idx, None)?;
        self.charge_lowering_set(gas_meter, module)?;
        Ok(module)
    }

    /// Brings a module this transaction has already charged for up to the
    /// state its policy requires before any of its functions can be lowered.
    ///
    /// Idempotent, so it can run on every call into an already-charged
    /// module: each policy resolves a memoized set and charges only members
    /// the transaction has not paid for, which on a repeat is none of them.
    fn ensure_ready_for_lowering(
        &self,
        gas_meter: &mut GasMeter,
        module: &'guard LoadedModule,
    ) -> VMResult<()> {
        match &self.policy {
            LoadingPolicy::Lazy(LoweringPolicy::Lazy) => Ok(()),
            LoadingPolicy::Lazy(LoweringPolicy::Eager) => {
                self.charge_lowering_set(gas_meter, module)
            },
            LoadingPolicy::Package => {
                // The module can have been charged for by a layout-only
                // side-load earlier in the transaction, which brings in no
                // siblings. Load the full package now.
                let id = self.guard.arena_ref_for_module_id(module.id());
                self.load_package(gas_meter, id).map(drop)
            },
        }
    }

    /// Charges for every member of MS(module) this transaction has not paid
    /// for yet, computing the set first if it is not known. The module is a
    /// member of its own set, so this is also what charges for it.
    fn charge_lowering_set(
        &self,
        gas_meter: &mut GasMeter,
        module: &'guard LoadedModule,
    ) -> VMResult<()> {
        let ms = match module.mandatory_dependencies_if_known() {
            Some(ms) => ms.as_ref(),
            None => self.compute_mandatory_set(module)?,
        };
        self.charge_and_load(gas_meter, ms, None)
    }

    /// Walks the module's lowering type closure and installs the resulting
    /// mandatory set, returning the set that won the race.
    fn compute_mandatory_set(
        &self,
        module: &'guard LoadedModule,
    ) -> VMResult<&'guard [InternedModuleId]> {
        let mut walker = LoweringContext::new(self);
        walker.discovered_seen.insert(module.id());
        walker.discovered.push(module.id());

        // Per-function lowering re-walks types and rebuilds its own
        // descriptor map; only the side-effecting publish-to-guard
        // matters here.
        let _ = try_discover_types_for_lowering_in_module(&mut walker, self.guard, module.ir())?;
        Ok(module.set_mandatory_dependencies(walker.discovered.into()))
    }

    /// Fetches, deserializes, and verifies the module from storage, returning
    /// it alongside its deterministic cost (byte length).
    fn get_verified_module_from_storage(
        &self,
        id: ArenaRef<'guard, ModuleId>,
    ) -> VMResult<(ModuleIR, u64)> {
        let bytes = self
            .module_provider
            .get_module_bytes(id.address(), id.name())?
            .ok_or_else(|| LoaderError::ModuleNotFound {
                address: *id.address(),
                name: id.name().to_string(),
            })?;
        // TODO(metering): placeholder cost model — byte length of the module. Replace
        // with a proper cost function (bucketed by size, verifier cost, etc.).
        let cost = bytes.len() as u64;
        let compiled_module = self.module_provider.deserialize_module(&bytes)?;
        self.module_provider.verify_module(&compiled_module)?;
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
        deps: Option<Arc<[InternedModuleId]>>,
    ) -> VMResult<&'guard LoadedModule> {
        let (module_ir, cost) = self.get_verified_module_from_storage(id)?;
        let module = LoadedModule::new(module_ir, cost, deps, self.guard)
            .map_err(|e| VMInternalError::new(LoaderError::GlobalContext(e)))?;
        self.guard
            .insert_module(module)
            .map_err(|e| VMInternalError::new(LoaderError::GlobalContext(e)))
    }

    /// Loads every module in `ids` this transaction has not charged for yet,
    /// records it as charged, and charges their costs as a single sum.
    /// `package`, when set, is installed as the mandatory set of every module
    /// built here.
    ///
    /// The sum must stay a single charge: a gas meter short of the total
    /// deducts nothing, so splitting it would leave a different balance on
    /// out-of-gas.
    ///
    /// A module is recorded before it is loaded, so a load that fails leaves
    /// the module recorded but absent from the table. That pair is how a later
    /// resolution tells a failed load apart from one never attempted.
    fn charge_and_load(
        &self,
        gas_meter: &mut GasMeter,
        ids: &[InternedModuleId],
        package: Option<&Arc<[InternedModuleId]>>,
    ) -> VMResult<()> {
        let mut loading_cost = 0u64;
        for &module_id in ids {
            let idx = self.module_idx(module_id)?;
            if !self.guard.mark_charged(idx) {
                continue;
            }
            let module = self.get_or_build(idx, package)?;
            loading_cost = loading_cost.saturating_add(module.cost());
        }
        gas_meter.charge(loading_cost)?;
        Ok(())
    }

    /// Returns the cached module, building and installing it from storage on a
    /// cache miss. `package`, when set, is installed as the module's mandatory
    /// set instead of deriving one from the policy.
    fn get_or_build(
        &self,
        idx: ModuleIdx,
        package: Option<&Arc<[InternedModuleId]>>,
    ) -> VMResult<&'guard LoadedModule> {
        if let Some(module) = self.guard.module_at(idx) {
            return Ok(module);
        }
        let id = self.module_id_at(idx)?;
        let deps = match package {
            Some(package) => Some(package.clone()),
            None => self.build_mandatory_dependencies_for_id(id)?,
        };
        self.build_and_insert_module_ir(id, deps)
    }

    /// The index of `module_id`, minting one if this transaction is the first
    /// to name it.
    fn module_idx(&self, module_id: InternedModuleId) -> VMResult<ModuleIdx> {
        self.guard
            .module_idx(module_id)
            .map_err(|e| VMInternalError::new(LoaderError::GlobalContext(e)))
    }

    /// The ID the index was minted for. Every index the loader holds came from
    /// the table, which creates the row and its ID together.
    fn module_id_at(&self, idx: ModuleIdx) -> VMResult<ArenaRef<'guard, ModuleId>> {
        match self.guard.module_id_at(idx) {
            Some(module_id) => Ok(self.guard.arena_ref_for_module_id(module_id)),
            None => invariant_violation!(ModuleIndexNotInTable),
        }
    }

    /// The module at `idx`, which this transaction has already charged for and
    /// so must be loaded.
    fn loaded_module_at(&self, idx: ModuleIdx) -> VMResult<&'guard LoadedModule> {
        match self.guard.module_at(idx) {
            Some(module) => Ok(module),
            None => invariant_violation!(ModuleNotLoaded),
        }
    }
}

/// Records the defining module of every nominal type occurring in `ty`. The
/// result may repeat a module; the second occurrence is already charged for and
/// so costs nothing.
///
/// TODO(metering): unbounded recursion, same family as the `TODO(metering)` on
/// `mono_move_core::types::is_closed_type`.
fn collect_nominal_modules(ty: InternedType, out: &mut Vec<InternedModuleId>) {
    match view_type(ty) {
        Type::Nominal {
            module_id, ty_args, ..
        } => {
            out.push(*module_id);
            for &ty_arg in view_type_list(*ty_args) {
                collect_nominal_modules(ty_arg, out);
            }
        },
        Type::Vector { elem } => collect_nominal_modules(*elem, out),
        Type::ImmutRef { inner } | Type::MutRef { inner } => collect_nominal_modules(*inner, out),
        Type::Function { args, results, .. } => {
            for &ty in view_type_list(*args).iter().chain(view_type_list(*results)) {
                collect_nominal_modules(ty, out);
            }
        },
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
        | Type::TypeParam { .. } => {},
    }
}

/// Collects the modules visited during a lowering requirements calculation.
struct LoweringContext<'a, 'guard, 'ctx> {
    loader: &'a Loader<'guard, 'ctx>,
    /// All modules needed for lowering of this function, ordered based on the
    /// specializer DFS type traversal.
    discovered: Vec<InternedModuleId>,
    discovered_seen: UnorderedSet<InternedModuleId>,
}

impl<'a, 'guard, 'ctx> LoweringContext<'a, 'guard, 'ctx> {
    fn new(loader: &'a Loader<'guard, 'ctx>) -> Self {
        Self {
            loader,
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
    ) -> VMResult<Option<NominalFields>> {
        // The walk does not record what it visits: the caller charges for
        // everything handed back in `discovered`, and a record here would make
        // that charge skip it.
        let idx = self.loader.module_idx(*module_id)?;
        let module = self.loader.get_or_build(idx, None)?;

        // Accumulate visited modules so that we can construct mandatory set
        // for the root module later.
        if self.discovered_seen.insert(*module_id) {
            self.discovered.push(*module_id);
        }

        Ok(module.ir().module.interned_fields(*nominal_name).cloned())
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

    fn publish_layout(&self, layout: ValueLayout) -> Option<LayoutId> {
        self.loader.guard.publish_layout(layout)
    }

    fn framework_symbols(&self) -> &FrameworkSymbols {
        self.loader.guard.framework_symbols()
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
