// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Implementation of loader to load modules from storage into the long-living
//! cache and per-transaction read-set with deterministic gas charging.

use crate::{
    module_provider::ModuleProvider,
    read_set::{ExecutableRead, ExecutableReadSet},
};
use anyhow::{anyhow, bail};
use fxhash::FxHashMap;
use mono_move_core::{Executable, ExecutableId, ExecutableSlot, MandatoryDependencies};
use mono_move_gas::GasMeter;
use mono_move_global_context::{ArenaRef, ExecutionGuard};
use mono_move_orchestrator::ExecutableBuilder;
use move_binary_format::{access::ModuleAccess, CompiledModule};

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
                    Eager => bail!("Eager lowering is currently not supported"),
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
                match load_content(self.guard, *slot) {
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

        let mut pending = Vec::with_capacity(names.len());
        for name in names {
            let member_id = self
                .guard
                .intern_address_name(id.address(), name.as_ident_str());
            let slot = self.guard.get_or_create_slot(member_id);
            let (module, cost) = self.get_verified_module_from_storage(member_id)?;
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
}

/// Loads the current executable content behind `slot`, or `None` if the
/// slot is empty. Safe while an execution guard is held: slot and content
/// are freed only under maintenance, which execution excludes. The guard
/// reference anchors `'guard` so the returned reference cannot outlive it.
fn load_content<'guard, 'ctx>(
    _guard: &'guard ExecutionGuard<'ctx>,
    slot: ExecutableSlot,
) -> Option<&'guard Executable> {
    unsafe { slot.as_ref_unchecked().load().map(|p| p.as_ref_unchecked()) }
}
