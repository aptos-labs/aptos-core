// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

pub mod compiled_module_viewer;
pub mod dependency_graph;
pub mod layout;

use crate::dependency_graph::DependencyGraph;
use anyhow::{anyhow, Result};
use move_binary_format::{access::ModuleAccess, file_format::CompiledModule};
use move_core_types::language_storage::ModuleId;
use std::collections::BTreeMap;

/// Set of Move modules indexed by module Id
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Modules<'a>(BTreeMap<ModuleId, &'a CompiledModule>);

impl<'a> Modules<'a> {
    /// Construct a set of modules from a slice `modules`.
    /// Panics if `modules` contains duplicates
    pub fn new(modules: impl IntoIterator<Item = &'a CompiledModule>) -> Self {
        let mut map = BTreeMap::new();
        for m in modules {
            assert!(
                map.insert(m.self_id(), m).is_none(),
                "Duplicate module found"
            );
        }
        Modules(map)
    }

    /// Return all modules in this set
    pub fn iter_modules(&self) -> Vec<&CompiledModule> {
        self.0.values().copied().collect()
    }

    /// Return all modules in this set
    pub fn iter_modules_owned(&self) -> Vec<CompiledModule> {
        self.iter_modules().into_iter().cloned().collect()
    }

    /// Compute a dependency graph for `self`
    pub fn compute_dependency_graph(&self) -> DependencyGraph<'_> {
        DependencyGraph::new(self.0.values().copied())
    }

    /// Return the backing map of `self`
    pub fn get_map(&self) -> &BTreeMap<ModuleId, &CompiledModule> {
        &self.0
    }

    /// Return the bytecode for the module bound to `module_id`
    pub fn get_module(&self, module_id: &ModuleId) -> Result<&CompiledModule> {
        self.0
            .get(module_id)
            .copied()
            .ok_or_else(|| anyhow!("Can't find module {:?}", module_id))
    }

    /// Return the immediate dependencies for `module_id`
    pub fn get_immediate_dependencies(&self, module_id: &ModuleId) -> Result<Vec<&CompiledModule>> {
        self.get_module(module_id)?
            .immediate_dependencies()
            .into_iter()
            .map(|mid| self.get_module(&mid))
            .collect::<Result<Vec<_>>>()
    }

    fn get_transitive_dependencies_(
        &'a self,
        all_deps: &mut Vec<&'a CompiledModule>,
        module: &'a CompiledModule,
    ) -> Result<()> {
        let next_deps = module.immediate_dependencies();
        all_deps.push(module);
        for next in next_deps {
            let next_module = self.get_module(&next)?;
            self.get_transitive_dependencies_(all_deps, next_module)?;
        }
        Ok(())
    }

    /// Return the transitive dependencies for `module_id`
    pub fn get_transitive_dependencies(
        &self,
        module_id: &ModuleId,
    ) -> Result<Vec<&CompiledModule>> {
        let mut all_deps = vec![];
        for dep in self.get_immediate_dependencies(module_id)? {
            self.get_transitive_dependencies_(&mut all_deps, dep)?;
        }
        Ok(all_deps)
    }
}
