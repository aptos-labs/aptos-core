// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Loaded module — what the module cache stores. Stores the polymorphic IR
//! with the lowered monomorphic functions and generic function instantiations.

use anyhow::Result;
use aptos_types::vm::module_metadata::{get_metadata, get_randomness_annotation};
use mono_move_core::{
    intern_struct_tag,
    interner::{InternedIdentifier, InternedModuleId},
    types::{view_type, InternedType, InternedTypeList, Type},
    Function, FunctionDefinitionIndex, FunctionPtr, Interner, NominalFields, PreparedModule,
};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{FunctionAttribute, StructDefinitionIndex, Visibility},
};
use move_core_types::identifier::IdentStr;
use parking_lot::Mutex;
use shared_dsa::{Entry, UnorderedMap, UnorderedSet};
use specializer::{FunctionIR, ModuleIR};
use std::sync::{Arc, OnceLock};

/// Lowered code for a single function instance, paired with the modules the
/// lowering required.
pub struct FunctionSlot {
    pub function: FunctionPtr,
    pub mandatory_dependencies: Arc<[InternedModuleId]>,
}

impl FunctionSlot {
    /// Returns a new slot owning the monomorphic function with its mandatory
    /// dependencies.
    pub fn new(function: Function, mandatory_dependencies: Arc<[InternedModuleId]>) -> Self {
        Self {
            function: FunctionPtr::new(Box::new(function)),
            mandatory_dependencies,
        }
    }
}

/// Result of looking up a function's polymorphic IR by name. See
/// [`LoadedModule::get_function_ir`].
pub enum FunctionIrLookup<'a> {
    /// The function is not defined in this module.
    NotDefined,
    /// The function is a native: defined, but has no IR.
    Native,
    /// The function's IR.
    Ir(&'a FunctionIR),
}

/// A loaded module: polymorphic IR and lazily lowered monomorphic functions.
pub struct LoadedModule {
    /// Polymorphic stackless IR.
    ir: ModuleIR,
    /// Deterministic load cost recorded at insertion time.
    cost: u64,
    /// Modules that must be loaded together with this one. Empty until the
    /// loading policy computes the set; a package load fills it at
    /// construction.
    ///
    /// # Invariants
    ///   1. Under LL the cell is always empty.
    ///   2. For shallow layout loads, the cell is created empty.
    ///   3. Under EL the loader fills it for module M once MS(M) has been
    ///      computed. Shallow side-loads stay with an unset cell until they
    ///      require lowering.
    ///   4. Filled entries always include self.
    mandatory_dependencies: OnceLock<Arc<[InternedModuleId]>>,
    /// Lowered code for the module's non-generic functions, one slot per
    /// function name. Filled on first call.
    ///
    /// # Invariants
    ///
    /// 1. One entry per non-generic function defined in this module.
    /// 2. Generic functions never appear here — they live in
    ///    [`Self::instantiated_functions`].
    functions: UnorderedMap<InternedIdentifier, OnceLock<FunctionSlot>>,
    /// Maps function's name to its index in file format (to query its IR).
    function_indices: UnorderedMap<InternedIdentifier, usize>,
    /// Lowered code for generic functions, one slot per (name, type
    /// arguments) pair. Filled on first call with that instantiation.
    ///
    /// # Invariants
    ///
    /// 1. The type argument list in each key is non-empty. Non-generic
    ///    functions are stored in [`Self::functions`].
    /// 2. The type arguments are fully concrete — they're the actual
    ///    runtime types the function was monomorphized for.
    // TODO(cleanup): revisit data structure used for actual monomorphized function storage.
    instantiated_functions:
        Mutex<UnorderedMap<(InternedIdentifier, InternedTypeList), FunctionSlot>>,
    /// Maps the name of a group member defined in this module (a nominal type)
    /// to its group container type (also a nominal type). Built once when the
    /// module is loaded. If struct's or enum's name is not in the map, then it
    /// is not a resource group member.
    resource_group_members: UnorderedMap<InternedIdentifier, InternedType>,
    /// Names of the functions carrying the `#[randomness]` annotation. Built
    /// once when the module is loaded.
    randomness_annotated: UnorderedSet<InternedIdentifier>,
    /// Indexed by struct definition: whether the compiler-generated pack API
    /// of the struct or enum is fully public, see
    /// [`Self::has_public_pack_api`]. Built once when the module is loaded.
    public_pack_apis: Box<[bool]>,
}

impl LoadedModule {
    pub fn new(
        ir: ModuleIR,
        cost: u64,
        mandatory_dependencies: Option<Arc<[InternedModuleId]>>,
        interner: &impl Interner,
    ) -> Result<Box<Self>> {
        let mandatory_dependencies = match mandatory_dependencies {
            Some(deps) => OnceLock::from(deps),
            None => OnceLock::new(),
        };
        let mut functions = UnorderedMap::with_capacity(ir.functions.len());
        let mut function_indices = UnorderedMap::with_capacity(ir.functions.len());
        let mut randomness_annotated = UnorderedSet::new();

        debug_assert_eq!(
            ir.functions.len(),
            ir.module.function_defs.len(),
            "ModuleIR must carry one entry per function definition"
        );
        // `zip`, not indexing: a length mismatch drops entries, degrading to a
        // failed name lookup.
        for (idx, (func_ir, fdef)) in ir
            .functions
            .iter()
            .zip(&ir.module.function_defs)
            .enumerate()
        {
            let name_idx = ir.module.function_handle_at(fdef.function).name;
            let name = ir.module.interned_identifier_at(name_idx);
            // Every definition, natives included, resolves by name; only
            // those with IR get a lowered-code slot.
            function_indices.insert(name, idx);
            if func_ir.is_some() {
                functions.insert(name, OnceLock::new());
            }
            let name_str = ir.module.identifier_at(name_idx).as_str();
            if get_randomness_annotation(name_str, &ir.module.metadata).is_some() {
                randomness_annotated.insert(name);
            }
        }
        let resource_group_members = Self::build_resource_group_members(&ir, interner)?;
        let public_pack_apis = Self::build_public_pack_apis(&ir);
        Ok(Box::new(Self {
            ir,
            cost,
            mandatory_dependencies,
            functions,
            function_indices,
            instantiated_functions: Mutex::new(UnorderedMap::new()),
            resource_group_members,
            randomness_annotated,
            public_pack_apis,
        }))
    }

    /// Builds the per-definition answer to [`Self::has_public_pack_api`] in
    /// one pass over the module's functions.
    ///
    /// Only two properties are checked here: that a function carries the
    /// `Pack` or `PackVariant` attribute, and that it is public. Everything
    /// else about a pack function is already guaranteed by the bytecode
    /// verifier's struct API checker.
    fn build_public_pack_apis(ir: &ModuleIR) -> Box<[bool]> {
        let module = &ir.module;
        // One flag per pack function the definition needs: one for a struct,
        // one per variant for an enum.
        let mut packable: Vec<Vec<bool>> = module
            .struct_defs()
            .iter()
            .map(|def| {
                let name =
                    module.interned_identifier_at(module.struct_handle_at(def.struct_handle).name);
                let num_variants = match module.interned_fields(name) {
                    Some(NominalFields::Enum(variants)) => variants.len(),
                    Some(NominalFields::Struct(_)) | None => 1,
                };
                vec![false; num_variants]
            })
            .collect();
        for def in module.function_defs() {
            if def.visibility != Visibility::Public {
                continue;
            }
            let handle = module.function_handle_at(def.function);
            let Some(def_idx) =
                Self::packed_definition(module, module.interned_types_at(handle.return_))
            else {
                continue;
            };
            for attribute in &handle.attributes {
                let variant = match attribute {
                    FunctionAttribute::Pack => 0,
                    FunctionAttribute::PackVariant(tag) => usize::from(*tag),
                    FunctionAttribute::Persistent
                    | FunctionAttribute::ModuleLock
                    | FunctionAttribute::Unpack
                    | FunctionAttribute::UnpackVariant(_)
                    | FunctionAttribute::TestVariant(_)
                    | FunctionAttribute::BorrowFieldImmutable(_)
                    | FunctionAttribute::BorrowFieldMutable(_) => continue,
                };
                if let Some(flag) = packable[def_idx.0 as usize].get_mut(variant) {
                    *flag = true;
                }
            }
        }
        packable
            .iter()
            .map(|flags| flags.iter().all(|&packable| packable))
            .collect()
    }

    /// The definition in this module that a function with the given return
    /// types packs: the single nominal it returns, if this module defines it.
    /// For a function carrying a pack attribute, the bytecode verifier
    /// guarantees this is `Some`; the `None` paths only make a function
    /// without one cheap to skip.
    fn packed_definition(
        module: &PreparedModule,
        return_tys: &[InternedType],
    ) -> Option<StructDefinitionIndex> {
        let &[returned] = return_tys else {
            return None;
        };
        let Type::Nominal {
            module_id, name, ..
        } = view_type(returned)
        else {
            return None;
        };
        if *module_id != module.id() {
            return None;
        }
        module.interned_nominal_type_def_idx(*name)
    }

    /// Builds the group-member map from this module's metadata:
    ///
    ///   - For each struct/enum carrying a `#[resource_group_member]`
    ///     attribute, its interned name maps to the interned type of the
    ///     group container it belongs to.
    fn build_resource_group_members(
        ir: &ModuleIR,
        interner: &impl Interner,
    ) -> Result<UnorderedMap<InternedIdentifier, InternedType>> {
        let mut members = UnorderedMap::new();
        if let Some(metadata) = get_metadata(&ir.module.metadata) {
            for (struct_name, attributes) in &metadata.struct_attributes {
                if let Some(group_tag) = attributes
                    .iter()
                    .find_map(|attr| attr.get_resource_group_member())
                {
                    let name = interner.identifier_of(IdentStr::new(struct_name)?);
                    let container = intern_struct_tag(&group_tag, interner)?;
                    members.insert(name, container);
                }
            }
        }
        Ok(members)
    }

    /// Returns the polymorphic stackless IR.
    pub fn ir(&self) -> &ModuleIR {
        &self.ir
    }

    /// Returns the modules that must be loaded together with this one, or an
    /// empty slice if the set has not been computed yet.
    pub fn mandatory_dependencies(&self) -> &[InternedModuleId] {
        self.mandatory_dependencies
            .get()
            .map(|deps| deps.as_ref())
            .unwrap_or(&[])
    }

    /// Returns the mandatory dependencies only if the set has been computed,
    /// distinguishing that from a set that is known to be empty.
    pub fn mandatory_dependencies_if_known(&self) -> Option<&Arc<[InternedModuleId]>> {
        self.mandatory_dependencies.get()
    }

    /// Installs the mandatory dependencies, returning the installed set. A
    /// concurrent computation may win the race, in which case its set is
    /// returned and `deps` is dropped.
    pub fn set_mandatory_dependencies(&self, deps: Arc<[InternedModuleId]>) -> &[InternedModuleId] {
        self.mandatory_dependencies.get_or_init(|| deps)
    }

    /// Returns interned module ID of this module.
    pub fn id(&self) -> InternedModuleId {
        self.ir.module.id()
    }

    /// Returns the deterministic load cost for this module.
    pub fn cost(&self) -> u64 {
        self.cost
    }

    /// Returns the function slot for the given name where monomorphized code
    /// may or may not be installed, or `None` if the function is not found.
    pub fn get_function_slot(&self, name: InternedIdentifier) -> Option<&OnceLock<FunctionSlot>> {
        self.functions.get(&name)
    }

    /// Looks up the polymorphic IR for the function with the given name.
    pub fn get_function_ir(&self, name: InternedIdentifier) -> FunctionIrLookup<'_> {
        let Some(&idx) = self.function_indices.get(&name) else {
            return FunctionIrLookup::NotDefined;
        };
        match self.ir.functions.get(idx).and_then(|slot| slot.as_ref()) {
            Some(ir) => FunctionIrLookup::Ir(ir),
            None => FunctionIrLookup::Native,
        }
    }

    /// Definition index of the named function in this module. Covers natives
    /// as well as Move-body functions.
    pub fn function_def_idx(&self, name: InternedIdentifier) -> Option<FunctionDefinitionIndex> {
        self.function_indices
            .get(&name)
            .map(|&idx| FunctionDefinitionIndex(idx as u16))
    }

    /// Returns the function and its mandatory dependencies for the given
    /// instantiation. If the function has not been monomorphized yet, returns
    /// [`None`].
    pub fn get_instantiated_function_ptr(
        &self,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
    ) -> Option<(FunctionPtr, Arc<[InternedModuleId]>)> {
        self.instantiated_functions
            .lock()
            .get(&(name, ty_args))
            .map(|f| (f.function, f.mandatory_dependencies.clone()))
    }

    /// Inserts monomorphized function into instantiation cache, returning the
    /// pointer to the inserted function. If the slot for the function is
    /// occupied (concurrent insertion), is a no-op and returns the existing
    /// pointer.
    pub fn set_instantiated_function(
        &self,
        name: InternedIdentifier,
        ty_args: InternedTypeList,
        function: Function,
        function_ms: Arc<[InternedModuleId]>,
    ) -> FunctionPtr {
        match self.instantiated_functions.lock().entry((name, ty_args)) {
            Entry::Occupied(e) => e.get().function,
            Entry::Vacant(e) => e.insert(FunctionSlot::new(function, function_ms)).function,
        }
    }

    /// Returns the group container type the named struct or enum is a member
    /// of, or [`None`] if it is not a resource-group member (it lives in its
    /// own storage slot).
    pub fn resource_group_of(&self, name: &InternedIdentifier) -> Option<InternedType> {
        self.resource_group_members.get(name).copied()
    }

    /// Whether the function `name` carries the `#[randomness]` annotation.
    pub fn has_randomness_annotation(&self, name: &InternedIdentifier) -> bool {
        self.randomness_annotated.contains(name)
    }

    /// Whether the struct or enum defined at `def_idx` can be packed from
    /// outside its module: a struct needs a public function carrying the
    /// `Pack` attribute, an enum one carrying `PackVariant` for every variant.
    /// Their well-formedness is the bytecode verifier's job, see
    /// [`Self::build_public_pack_apis`].
    pub fn has_public_pack_api(&self, def_idx: StructDefinitionIndex) -> bool {
        self.public_pack_apis
            .get(def_idx.0 as usize)
            .copied()
            .unwrap_or(false)
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
    // TODO(correctness): `FunctionPtr`s in other modules' `CallDirect` ops are only sound
    //   if callers are evicted with direct callees. (or their code is de-optimized).
    fn drop(&mut self) {
        self.functions.retain(|_, cell| {
            if let Some(slot) = cell.take() {
                // SAFETY: see impl-level comment — no aliases at drop time.
                unsafe { slot.function.free_unchecked() };
            }
            false
        });
        self.instantiated_functions.lock().for_each_value(|slot| {
            // SAFETY: see impl-level comment — no aliases at drop time.
            unsafe { slot.function.free_unchecked() };
        });
    }
}
