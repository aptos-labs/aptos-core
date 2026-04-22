// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Implementation of loader to load modules from storage into the long-living
//! cache and per-transaction read-set with deterministic gas charging.

use crate::{
    module_provider::ModuleProvider,
    read_set::{ExecutableRead, ExecutableReadSet},
};
use anyhow::{anyhow, bail};
use fxhash::{FxHashMap, FxHashSet};
use mono_move_core::{Executable, ExecutableId, ExecutableSlot, MandatoryDependencies};
use mono_move_gas::GasMeter;
use mono_move_global_context::{struct_info_at, ArenaRef, ExecutionGuard};
use mono_move_orchestrator::ExecutableBuilder;
use move_binary_format::{
    access::ModuleAccess,
    file_format::{SignatureToken, StructFieldInformation, StructHandleIndex},
    CompiledModule,
};
use std::collections::hash_map::Entry;

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
}

impl<'guard, 'ctx> Loader<'guard, 'ctx> {
    /// Creates a new loader. The provided [`ModuleProvider`] processes cache
    /// misses: fetch code from storage, deserialize and verify. Policy
    /// dictates how the code is loaded.
    pub fn new_with_policy(
        guard: &'guard ExecutionGuard<'ctx>,
        module_provider: &'guard dyn ModuleProvider,
        policy: LoadingPolicy,
    ) -> Self {
        Self {
            guard,
            module_provider,
            policy,
        }
    }

    /// Loads and returns the executable corresponding to the given ID,
    /// records it (and any policy-dictated mandatory dependencies) in the
    /// transaction's read-set, and charges gas for the load.
    ///
    /// # Precondition
    ///
    /// The code has not been added to the read-set yet.
    pub fn load(
        &self,
        read_set: &mut ExecutableReadSet<'guard>,
        gas_meter: &mut impl GasMeter,
        id: ArenaRef<'guard, ExecutableId>,
    ) -> anyhow::Result<&'guard Executable> {
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
}

//
// Only private APIs below.
// ------------------------

impl<'guard, 'ctx> Loader<'guard, 'ctx> {
    /// Loads only the code corresponding to the specified ID and charges
    /// gas for this code instance.
    fn load_lazy_with_lazy_lowering(
        &self,
        read_set: &mut ExecutableReadSet<'guard>,
        gas_meter: &mut impl GasMeter,
        id: ArenaRef<'guard, ExecutableId>,
    ) -> anyhow::Result<&'guard Executable> {
        if let Some(executable) = self.guard.get_executable(id) {
            self.record_loaded_and_charge(read_set, gas_meter, executable)?;
            return Ok(executable);
        }

        let (module, cost) = self.get_verified_module_from_storage(id)?;
        let executable = self.build_and_insert(&module, cost, MandatoryDependencies::empty())?;
        self.record_loaded_and_charge(read_set, gas_meter, executable)?;
        Ok(executable)
    }

    /// Loads the code corresponding to the specified ID and all other
    /// modules in the same package. Gas is charged for the whole package
    /// whether it was cache miss or hit.
    fn load_package(
        &self,
        read_set: &mut ExecutableReadSet<'guard>,
        gas_meter: &mut impl GasMeter,
        id: ArenaRef<'guard, ExecutableId>,
    ) -> anyhow::Result<&'guard Executable> {
        if let Some(executable) = self.guard.get_executable(id) {
            // `slots` covers all package members, including self. Probe
            // every slot before touching the read-set: a concurrent
            // miss-path worker may have populated `id`'s slot but not yet
            // all sibling slots. In that window we fall through to the
            // miss path, which re-verifies and re-inserts.
            // `insert_executable` returns the canonical winner on CAS
            // loss, so already-cached members are not duplicated.
            let slots = executable.mandatory_dependencies().slots();
            let mut members = Vec::with_capacity(slots.len());
            let mut all_present = true;
            for slot in slots {
                match load_content(self.guard, slot) {
                    Some(member) => members.push(member),
                    None => {
                        all_present = false;
                        break;
                    },
                }
            }

            if all_present {
                let mut total = 0u64;
                for member in members {
                    let member_id = self.guard.arena_ref_for_executable_id(member.id());
                    total = total.saturating_add(member.cost());
                    read_set.record(member_id, ExecutableRead::Loaded(member))?;
                }
                gas_meter.charge(total)?;
                return Ok(executable);
            }
            // Fall through to the miss path.
        }

        let names = self
            .module_provider
            .get_same_package_modules(id.address(), id.name())?;

        let mut total = 0u64;
        let mut pending = Vec::with_capacity(names.len());
        for name in names {
            let member_id = self
                .guard
                .intern_address_name(id.address(), name.as_ident_str());
            let slot = self.guard.get_or_create_slot(member_id);
            let (module, cost) = self.get_verified_module_from_storage(member_id)?;
            total = total.saturating_add(cost);
            pending.push((slot, module, cost));
        }

        let pending = self.topological_ordering(pending);

        let package_slots = pending.iter().map(|(slot, _, _)| *slot).collect::<Vec<_>>();
        let mandatory_deps = MandatoryDependencies::package(package_slots);

        let mut total = 0u64;
        for (_, module, cost) in &pending {
            let executable = self.build_and_insert(module, *cost, mandatory_deps.clone())?;
            let id = self.guard.arena_ref_for_executable_id(executable.id());
            read_set.record(id, ExecutableRead::Loaded(executable))?;
            total = total.saturating_add(executable.cost());
        }

        gas_meter.charge(total)?;

        Ok(read_set
            .get(id)
            .expect("Every executable was recorded in read-set"))
    }

    /// Loads the code corresponding to the specified ID together with the
    /// transitive closure of modules reachable through its struct
    /// definitions. Enum modules referenced from the target's struct
    /// handles join the closure but their variants are not walked.
    fn load_lazy_with_eager_lowering(
        &self,
        read_set: &mut ExecutableReadSet<'guard>,
        gas_meter: &mut impl GasMeter,
        id: ArenaRef<'guard, ExecutableId>,
    ) -> anyhow::Result<&'guard Executable> {
        // TODO: the simplified `MandatoryDependencies` API cannot distinguish
        // "closure not yet computed" from "empty closure". We conservatively
        // treat any non-empty set as the installed closure. When the eager
        // policy stabilises, restore the deferred closure state so closure
        // members can be upgraded in place instead of being rebuilt.
        if let Some(executable) = self.guard.get_executable(id)
            && !executable.mandatory_dependencies().slots().is_empty()
        {
            return self.record_and_charge_mandatory_deps(read_set, gas_meter, executable);
        }

        let (pending, target_slots) = self.walk_struct_closure(id)?;
        let pending = self.topological_ordering(pending);

        // Build non-target members first so the target can be built last
        // with its closure already set. Non-target members are inserted
        // with empty mandatory deps — they will be rebuilt if later loaded
        // as their own eager target.
        // TODO: rebuild is wasted work. Revisit once the deferred closure
        // state is restored or the cache can upgrade a member's deps in
        // place.
        let mut target_module_cost = None;
        for (_, module, cost) in pending {
            let module_id = self
                .guard
                .intern_address_name(module.self_addr(), module.self_name());
            if module_id == id {
                target_module_cost = Some((module, cost));
                continue;
            }
            self.build_and_insert(&module, cost, MandatoryDependencies::empty())?;
        }

        let (target_module, target_cost) =
            target_module_cost.expect("walk_struct_closure always includes the target in pending");
        let target_deps = MandatoryDependencies::package(target_slots.into_vec());
        let target = self.build_and_insert(&target_module, target_cost, target_deps)?;

        self.record_and_charge_mandatory_deps(read_set, gas_meter, target)
    }

    /// Orders modules leaves-first by inter-member import dependencies,
    /// so that building in sequence finds each referenced sibling already
    /// interned. Edges into modules outside the given modules are ignored.
    ///
    /// # Note
    ///
    /// Move publish guarantees that packages contain no dependency cycles, so
    /// the result is always topological.
    // TODO: double-check that publish rejects packages with cycles.
    fn topological_ordering(
        &self,
        modules: Vec<(ExecutableSlot, CompiledModule, u64)>,
    ) -> Vec<(ExecutableSlot, CompiledModule, u64)> {
        let id_to_idx = modules
            .iter()
            .enumerate()
            .map(|(i, (_, m, _))| {
                (
                    self.guard.intern_address_name(m.self_addr(), m.self_name()),
                    i,
                )
            })
            .collect::<FxHashMap<_, _>>();

        let deps = modules
            .iter()
            .map(|(_, m, _)| {
                m.immediate_dependencies_iter()
                    .filter_map(|(addr, name)| {
                        id_to_idx
                            .get(&self.guard.intern_address_name(addr, name))
                            .copied()
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let mut visited = vec![false; modules.len()];
        let mut post_order = Vec::with_capacity(modules.len());

        for root in 0..modules.len() {
            if visited[root] {
                continue;
            }
            visited[root] = true;
            let mut stack = vec![(root, 0)];

            while let Some(&(idx, cursor)) = stack.last() {
                if cursor < deps[idx].len() {
                    stack.last_mut().expect("stack is non-empty").1 = cursor + 1;
                    let dep_idx = deps[idx][cursor];
                    if !visited[dep_idx] {
                        visited[dep_idx] = true;
                        stack.push((dep_idx, 0));
                    }
                } else {
                    post_order.push(idx);
                    stack.pop();
                }
            }
        }

        let mut taken = modules.into_iter().map(Some).collect::<Vec<_>>();
        post_order
            .into_iter()
            .map(|i| taken[i].take().expect("each index emitted exactly once"))
            .collect()
    }

    /// Builds an executable for the compiled module and inserts it into the
    /// cache, returning the canonical pointer.
    fn build_and_insert(
        &self,
        module: &CompiledModule,
        cost: u64,
        deps: MandatoryDependencies,
    ) -> anyhow::Result<&'guard Executable> {
        let executable = ExecutableBuilder::new(self.guard, module)
            .with_cost(cost)
            .with_mandatory_dependencies(deps)
            .build()?;
        self.guard.insert_executable(executable)
    }

    /// Fetches, deserializes, and verifies the module from storage, returning
    /// it alongside its deterministic cost (byte length).
    fn get_verified_module_from_storage(
        &self,
        id: ArenaRef<'guard, ExecutableId>,
    ) -> anyhow::Result<(CompiledModule, u64)> {
        let bytes = self
            .module_provider
            .get_module_bytes(id.address(), id.name())?
            .ok_or_else(|| anyhow!("Linker error"))?;
        // TODO: placeholder cost model — byte length of the module. Replace
        // with a proper cost function (bucketed by size, verifier cost, etc.).
        let cost = bytes.len() as u64;
        let compiled = self.module_provider.deserialize_module(&bytes)?;
        self.module_provider.verify_module(&compiled)?;
        Ok((compiled, cost))
    }

    /// Records a single executable in the read-set and charges its cost.
    fn record_loaded_and_charge(
        &self,
        read_set: &mut ExecutableReadSet<'guard>,
        gas_meter: &mut impl GasMeter,
        executable: &'guard Executable,
    ) -> anyhow::Result<()> {
        let id = self.guard.arena_ref_for_executable_id(executable.id());
        // Always record the read before charging, so that if charging fails,
        // the read is part of the set for later Block-STM validation.
        read_set.record(id, ExecutableRead::Loaded(executable))?;
        gas_meter.charge(executable.cost())?;
        Ok(())
    }

    /// Records `target` as `Charged` together with every member of its
    /// transitive struct closure as `Cached`. Charges target's cost only
    /// if it was not already in the read-set; charges each closure member
    /// only if it was not already in the read-set.
    ///
    /// If `target` was previously recorded as `Cached` (sub-member of an
    /// earlier `EagerLowering` target), recording it again as `Charged`
    /// upgrades the entry in place without double-charging.
    fn record_and_charge_mandatory_deps(
        &self,
        read_set: &mut ExecutableReadSet<'guard>,
        gas_meter: &mut impl GasMeter,
        target: &'guard Executable,
    ) -> anyhow::Result<&'guard Executable> {
        let id = self.guard.arena_ref_for_executable_id(target.id());

        // TODO: the simplified read-set has a single `Loaded` variant and no
        // `contains` / upgrade path, so we cannot distinguish a prior cached
        // record from a charged record. We skip re-recording entries already
        // in the set and avoid double-charging them. Restore the Cached /
        // Charged variants (with an upgrade op) when the eager policy lands.
        let mut total: u64 = 0;
        if read_set.get(id).is_none() {
            total = total.saturating_add(target.cost());
            read_set.record(id, ExecutableRead::Loaded(target))?;
        }

        for slot in target.mandatory_dependencies().slots() {
            let member = load_content(self.guard, slot).expect("closure member must be loaded");
            let member_id = self.guard.arena_ref_for_executable_id(member.id());
            if read_set.get(member_id).is_none() {
                total = total.saturating_add(member.cost());
                read_set.record(member_id, ExecutableRead::Loaded(member))?;
            }
        }
        gas_meter.charge(total)?;
        Ok(target)
    }

    /// Walks the target's transitive struct-definition closure and returns
    /// everything `load_lazy_with_eager_lowering` needs after the walk:
    ///
    /// - `pending` — cache-miss members paired with their slot, compiled
    ///   form, and cost. The caller orders these leaves-first via
    ///   [`Self::topological_ordering`] and then builds each. Cache-hit
    ///   members are not included (their executables already exist).
    /// - `target_slots` — the closure's member slots, target excluded. The
    ///   caller passes this to the target's
    ///   `MandatoryDependencies::set_struct_closure`.
    ///
    /// The walk recurses into struct field signatures only. When it reaches
    /// an enum, the enum's module stays in the closure (so its `Type::Enum`
    /// identity is interned when the module is built and charged like any
    /// other member), but the walk does not descend into the enum's
    /// variant fields — enums are pointer-sized and the target's layout
    /// doesn't depend on variant content.
    fn walk_struct_closure(
        &self,
        target_id: ArenaRef<'guard, ExecutableId>,
    ) -> anyhow::Result<(
        Vec<(ExecutableSlot, CompiledModule, u64)>,
        Box<[ExecutableSlot]>,
    )> {
        let mut closure = FxHashMap::default();
        let mut visited_structs = FxHashSet::default();
        let mut stack = vec![];

        let (target, target_cost) = self.get_verified_module_from_storage(target_id)?;
        for (i, handle) in target.struct_handles().iter().enumerate() {
            if handle.module == target.self_handle_idx() {
                continue;
            }
            let idx = StructHandleIndex::new(i as u16);
            let (addr, module_name, struct_name) = struct_info_at(&target, idx);
            let dep_id = self.guard.intern_address_name(addr, module_name);
            let dep_struct_name = self.guard.intern_identifier(struct_name);
            stack.push((dep_id, dep_struct_name));
        }
        closure.insert(target_id, (target, target_cost));

        while let Some((mod_id, struct_name)) = stack.pop() {
            if !visited_structs.insert((mod_id, struct_name)) {
                continue;
            }

            let (module, _) = match closure.entry(mod_id) {
                Entry::Occupied(e) => e.into_mut(),
                Entry::Vacant(e) => {
                    let (compiled, cost) = self.get_verified_module_from_storage(mod_id)?;
                    e.insert((compiled, cost))
                },
            };

            let Some(struct_def) = module.struct_defs().iter().find(|def| {
                let handle = module.struct_handle_at(def.struct_handle);
                module.identifier_at(handle.name).as_str() == struct_name.as_str()
            }) else {
                continue;
            };

            let fields = match &struct_def.field_information {
                StructFieldInformation::Declared(fields) => fields,
                StructFieldInformation::DeclaredVariants(_) => continue,
                StructFieldInformation::Native => {
                    bail!("Native fields are deprecated");
                },
            };

            let mut indices = vec![];
            for field in fields {
                collect_struct_handle_indices(&field.signature.0, &mut indices);
            }
            for idx in indices {
                let (addr, module_name, struct_name) = struct_info_at(module, idx);
                let dep_id = self.guard.intern_address_name(addr, module_name);
                let dep_struct_name = self.guard.intern_identifier(struct_name);
                stack.push((dep_id, dep_struct_name));
            }
        }

        // Every reached module goes into `pending` and (unless it is the
        // target) also contributes its slot to `target_slots`. Cache-hit
        // members are rebuilt redundantly; `insert_executable` is
        // idempotent and returns the canonical pointer on race-loss.
        // TODO: optimize by filtering cache-hit members out of `pending`.
        let mut pending = Vec::with_capacity(closure.len());
        let mut target_slots = Vec::with_capacity(closure.len().saturating_sub(1));
        for (mod_id, (module, cost)) in closure {
            let slot = self.guard.get_or_create_slot(mod_id);
            if mod_id != target_id {
                target_slots.push(slot);
            }
            pending.push((slot, module, cost));
        }

        Ok((pending, target_slots.into_boxed_slice()))
    }
}

/// Loads the current executable content behind `slot`, or `None` if the
/// slot is empty. Safe while an execution guard is held: slot and content
/// are freed only under maintenance, which execution excludes. The guard
/// reference anchors `'guard` so the returned reference cannot outlive it.
fn load_content<'guard, 'ctx>(
    _guard: &'guard ExecutionGuard<'ctx>,
    slot: &ExecutableSlot,
) -> Option<&'guard Executable> {
    unsafe { slot.as_ref_unchecked().load().map(|p| p.as_ref_unchecked()) }
}

/// Walks signature token and records every struct handle index it transitively
/// references.
fn collect_struct_handle_indices(token: &SignatureToken, out: &mut Vec<StructHandleIndex>) {
    // TODO: Reimplement non-recursively.
    match token {
        SignatureToken::Struct(idx) => {
            out.push(*idx);
        },
        SignatureToken::StructInstantiation(idx, ty_args) => {
            out.push(*idx);
            for ty in ty_args.iter() {
                collect_struct_handle_indices(ty, out);
            }
        },
        SignatureToken::Vector(ty)
        | SignatureToken::Reference(ty)
        | SignatureToken::MutableReference(ty) => {
            collect_struct_handle_indices(ty, out);
        },
        SignatureToken::Function(args, rets, _) => {
            for ty in args.iter().chain(rets.iter()) {
                collect_struct_handle_indices(ty, out);
            }
        },
        SignatureToken::Bool
        | SignatureToken::U8
        | SignatureToken::U16
        | SignatureToken::U32
        | SignatureToken::U64
        | SignatureToken::U128
        | SignatureToken::U256
        | SignatureToken::I8
        | SignatureToken::I16
        | SignatureToken::I32
        | SignatureToken::I64
        | SignatureToken::I128
        | SignatureToken::I256
        | SignatureToken::Address
        | SignatureToken::Signer
        | SignatureToken::TypeParameter(_) => (),
    }
}
