// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Policy-driven loader over the executable cache.
//!
//! Each entry point drives one of the loading policies described in the
//! crate `README.md`: [`Loader::load_lazy`] and [`Loader::load_package`].
//! Both accept an already-interned [`ArenaRef<ExecutableId>`]; the caller
//! is responsible for interning.
//!
//! A third policy, `LazyWithTransitiveStructs`, is specified in the design
//! and will be added in a follow-up change.

use crate::{hooks::LoaderHooks, read_set::ExecutableReadSet};
use anyhow::{anyhow, Result};
use fxhash::FxHashMap;
use mono_move_core::{Executable, ExecutableId, ExecutableSlot, MandatoryDependencies};
use mono_move_gas::GasMeter;
use mono_move_global_context::{ArenaRef, ExecutionGuard};
use mono_move_orchestrator::ExecutableBuilder;
use move_binary_format::{access::ModuleAccess, CompiledModule};
use std::sync::Arc;

/// Describes how modules are loaded from storage.
pub enum LoadingPolicy {
    /// Loads one module at a time.
    Lazy,
    /// Loads one package at a time.
    Package,
}

/// Per-transaction code loader: loads code from the cache, charges gas on
/// load, handles cache misses, and records each loaded executable in the
/// transaction's read-set.
pub struct Loader<'guard, 'ctx> {
    guard: &'guard ExecutionGuard<'ctx>,
    hooks: &'guard dyn LoaderHooks,
    policy: LoadingPolicy,
}

impl<'guard, 'ctx> Loader<'guard, 'ctx> {
    /// Creates a new loader. Provided hooks are used to process cache misses:
    /// fetch code from storage, deserialize and verify. Policy dictates how
    /// the code is loaded.
    pub fn new_with_policy(
        guard: &'guard ExecutionGuard<'ctx>,
        hooks: &'guard dyn LoaderHooks,
        policy: LoadingPolicy,
    ) -> Self {
        Self {
            guard,
            hooks,
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
    ) -> Result<&'guard Executable> {
        match self.policy {
            LoadingPolicy::Lazy => self.load_lazy(read_set, gas_meter, id),
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
    fn load_lazy(
        &self,
        read_set: &mut ExecutableReadSet<'guard>,
        gas_meter: &mut impl GasMeter,
        id: ArenaRef<'guard, ExecutableId>,
    ) -> Result<&'guard Executable> {
        if let Some(executable) = self.guard.get_executable(id) {
            self.record_and_charge(read_set, gas_meter, executable)?;
            return Ok(executable);
        }

        let (module, cost) = self.get_verified_module_from_storage(id)?;
        let executable = self.build_and_insert(&module, cost, MandatoryDependencies::None)?;
        self.record_and_charge(read_set, gas_meter, executable)?;
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
    ) -> Result<&'guard Executable> {
        if let Some(executable) = self.guard.get_executable(id) {
            let slots = executable
                .mandatory_dependencies()
                .slots()
                .ok_or_else(|| anyhow!("Package policy must always set its slots at build time"))?;
            read_set.record(id, executable);
            let mut total = executable.cost();
            for slot in slots {
                // SAFETY: every cached package member has every sibling
                // slot populated, because Package loading installs the
                // whole member set before recording any of them.
                let other =
                    load_content(self.guard, *slot).expect("Package member slot must exist");
                let other_id = self.guard.arena_ref_for_executable_id(other.id());
                total = total.saturating_add(other.cost());
                read_set.record(other_id, other);
            }
            gas_meter.charge(total)?;
            return Ok(executable);
        }

        let names = self
            .hooks
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

        // Charge for the whole package up front, before any executable is
        // built or inserted into the cache. Otherwise a transaction with
        // budget just under the package cost would get all build work done
        // for free, leaving the results cached for future transactions.
        let total = pending
            .iter()
            .fold(0u64, |acc, (_, _, cost)| acc.saturating_add(*cost));
        gas_meter.charge(total)?;

        for i in 0..pending.len() {
            let siblings = pending
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != i)
                .map(|(_, (slot, _, _))| *slot)
                .collect::<Vec<_>>();
            let (_, module, cost) = &pending[i];
            let executable = self.build_and_insert(
                module,
                *cost,
                // TODO: maybe it is better to actually include self.
                MandatoryDependencies::Package(Arc::from(siblings)),
            )?;
            let exec_id = self.guard.arena_ref_for_executable_id(executable.id());
            read_set.record(exec_id, executable);
        }

        Ok(read_set
            .get(id)
            .expect("Every executable was recorded in read-set"))
    }

    /// Orders modules leaves-first by inter-member import dependencies,
    /// so that building in sequence finds each referenced sibling already
    /// interned. Edges into modules outside the gven modules are ignored.
    fn topological_ordering(
        &self,
        modules: Vec<(ExecutableSlot, CompiledModule, u64)>,
    ) -> Vec<(ExecutableSlot, CompiledModule, u64)> {
        let id_to_idx = modules
            .iter()
            .enumerate()
            .map(|(i, (_, m, _))| (self.guard.intern_module_id(&m.self_id()), i))
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
    ) -> Result<&'guard Executable> {
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
    ) -> Result<(CompiledModule, u64)> {
        let bytes = self
            .hooks
            .get_module_bytes(id.address(), id.name())?
            .ok_or_else(|| anyhow!("Linker error"))?;
        let cost = bytes.len() as u64;
        let compiled = self.hooks.deserialize_module(&bytes)?;
        self.hooks.verify_module(&compiled)?;
        Ok((compiled, cost))
    }

    /// Records a single executable in the read-set and charges its cost.
    fn record_and_charge(
        &self,
        read_set: &mut ExecutableReadSet<'guard>,
        gas_meter: &mut impl GasMeter,
        executable: &'guard Executable,
    ) -> Result<()> {
        let id = self.guard.arena_ref_for_executable_id(executable.id());
        read_set.record(id, executable);
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
