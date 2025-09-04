// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! Implements the "dead store elimination" transformation.
//!
//! This transformation should be run after the variable coalescing transformation,
//! as it removes the dead stores that variable coalescing may introduce.
//!
//! Prerequisite: the `LiveVarAnnotation` should already be computed by running the
//! `LiveVarAnalysisProcessor` in the `track_all_usages` mode.
//! Side effect: all annotations will be removed from the function target annotations.
//!
//! Given live variables and all their usages at each program point,
//! this transformation removes dead stores, i.e., assignments and loads to locals which
//! are not live afterwards (or are live only in dead code, making them effectively dead).
//! In addition, it also removes self-assignments, i.e., assignments of the form `x = x`.
//! One can also remove only those self-assignments where the definition is in the same block
//! before the self-assign by using `eliminate_all_self_assigns=false`.

use crate::pipeline::livevar_analysis_processor::LiveVarAnnotation;
use move_binary_format::file_format::CodeOffset;
use move_model::{ast::TempIndex, model::FunctionEnv};
use move_stackless_bytecode::{
    function_target::{FunctionData, FunctionTarget},
    function_target_pipeline::{FunctionTargetProcessor, FunctionTargetsHolder},
    stackless_bytecode::Bytecode,
};
use std::collections::{BTreeMap, BTreeSet};

/// A (reduced) def-use graph, where:
/// - each node is a code offset, representing a def and/or a use of a local.
/// - a forward edge (`children`) `a -> b` means that the def at `a` is used at `b`.
/// - each forward edge `a -> b` has a corresponding backward edge (`parents`) `a <- b`.
/// - def nodes that are tracked to be dead are marked as `defs_dead`.
/// - def nodes that are tracked to be alive (based on info known so far) are marked
///   as `defs_alive`. Some of these could eventually be moved to `defs_dead`.
///
/// Note that the *def* nodes are limited to side-effect-free instructions of the form:
/// - `Assign(dst, src)`
/// - `Load(dst, constant)`
/// This is a conservative over-approximation of side-effect-free instructions that
/// define a local.
///
/// The nodes representing *only uses* have no restrictions like for defs. These are
/// always leaves in the graph, and are never marked as dead.
///
/// A node can represent both a def and a use: such a node can only be of the
/// form `Assign(dst, src)`, which follows from the above restrictions.
///
/// Based on these restrictions, many code offsets in a function may not be present
/// in this graph: it is a reduced version of a typical def-use graph.
///
/// We use this graph to find side-effect-free defs, which can be removed safely if
/// they are not used later.
struct ReducedDefUseGraph {
    children: BTreeMap<CodeOffset, BTreeSet<CodeOffset>>,
    parents: BTreeMap<CodeOffset, BTreeSet<CodeOffset>>,
    defs_alive: BTreeSet<CodeOffset>,
    defs_dead: BTreeSet<CodeOffset>,
}

impl ReducedDefUseGraph {
    /// Get the dead stores that are safe to remove from the function `target`.
    /// If `eliminate_all_self_assigns` is true, all self-assignments are removed.
    pub fn dead_stores(target: &FunctionTarget, eliminate_all_self_assigns: bool) -> BTreeSet<u16> {
        Self {
            children: BTreeMap::new(),
            parents: BTreeMap::new(),
            defs_alive: BTreeSet::new(),
            defs_dead: BTreeSet::new(),
        }
        .run_stages(target, eliminate_all_self_assigns)
    }

    /// Run various stages to return the dead stores from `target`.
    /// If `eliminate_all_self_assigns` is true, all self-assignments are removed.
    fn run_stages(
        mut self,
        target: &FunctionTarget,
        eliminate_all_self_assigns: bool,
    ) -> BTreeSet<u16> {
        let code = target.get_bytecode();
        let live_vars = target
            .get_annotations()
            .get::<LiveVarAnnotation>()
            .expect("live variable annotation is a prerequisite");
        let mut self_assigns = Vec::new();
        // Stage 1: Incorporate all (restricted) defs and their uses into the graph.
        // Each (restricted) def is put either in `defs_alive` or `defs_dead.
        for (offset, instr) in code.iter().enumerate() {
            use Bytecode::*;
            match instr {
                Assign(_, dst, src, _) if dst == src => {
                    self_assigns.push(offset as CodeOffset);
                    self.incorporate_definition(*dst, offset as CodeOffset, live_vars);
                },
                Assign(_, dst, ..) | Load(_, dst, _) => {
                    self.incorporate_definition(*dst, offset as CodeOffset, live_vars);
                },
                _ => {},
            }
        }
        // Stage 2: Disconnect dead defs (which are guaranteed to be leaves) from the graph.
        // This is so that they don't prevent their parents from being dead (later below).
        for dead_def_leaf in self.defs_dead.clone() {
            self.disconnect_from_parents(dead_def_leaf);
        }
        // Stage 3: Let's disconnect self-assignments from the graph and kill them
        // (conditioned upon `eliminate_all_self_assigns`).
        for self_assign in self_assigns {
            let eliminate_this_self_assign = Self::should_eliminate_given_self_assign(
                self_assign,
                code,
                live_vars,
                eliminate_all_self_assigns,
            );
            if !eliminate_this_self_assign {
                continue;
            }
            let mut parents = self.disconnect_from_parents(self_assign);
            let mut children = self.disconnect_from_children(self_assign);
            // In case there is a cycle of self-assignments in the graph.
            parents.remove(&self_assign);
            children.remove(&self_assign);
            // A self-assignment is both a def and use (of the same local),
            // so its parents (if any) should be connected to all its uses (if any).
            for parent in parents.iter() {
                for child in children.iter() {
                    self.children.entry(*parent).or_default().insert(*child);
                    self.parents.entry(*child).or_default().insert(*parent);
                }
            }
            self.kill_def(self_assign);
        }
        // Stage 4: Start from the dead def leaves and remove them and their (now) dead parents
        // transitively.
        let mut def_leaves = self.compute_def_leaves();
        while let Some(leaf) = def_leaves.pop_last() {
            let parents = self.disconnect_from_parents(leaf);
            self.kill_def(leaf);
            // Parents are always defs, but they may have now become dead.
            for parent in parents {
                match self.children.get(&parent) {
                    Some(children) if children.is_empty() => {
                        def_leaves.insert(parent);
                    },
                    None => {
                        def_leaves.insert(parent);
                    },
                    _ => {},
                }
            }
        }
        // Note: the stage above does not eliminate a cycle of dead defs, tracked to be
        // fixed in #12400.
        self.defs_dead
    }

    fn kill_def(&mut self, def: CodeOffset) {
        self.defs_alive.remove(&def);
        self.defs_dead.insert(def);
    }

    /// Compute the set of defs (alive so far) that are leaves.
    fn compute_def_leaves(&self) -> BTreeSet<CodeOffset> {
        self.defs_alive
            .iter()
            .filter(|node| match self.children.get(*node) {
                Some(children) => children.is_empty(),
                None => true,
            })
            .copied()
            .collect()
    }

    /// Disconnect `child` from its parents and return the set of parents.
    fn disconnect_from_parents(&mut self, child: CodeOffset) -> BTreeSet<CodeOffset> {
        if let Some(parents) = self.parents.remove(&child) {
            for parent in parents.iter() {
                let children = self
                    .children
                    .get_mut(parent)
                    .expect("parent of a child must have children");
                children.remove(&child);
            }
            parents
        } else {
            BTreeSet::new()
        }
    }

    /// Disconnect `parent` from its children and return the set of children.
    fn disconnect_from_children(&mut self, parent: CodeOffset) -> BTreeSet<CodeOffset> {
        if let Some(children) = self.children.remove(&parent) {
            for child in children.iter() {
                let parents = self
                    .parents
                    .get_mut(child)
                    .expect("child of a parent must have parents");
                parents.remove(&parent);
            }
            children
        } else {
            BTreeSet::new()
        }
    }

    /// Incorporate a def of `local` at `offset` into the graph, using the `live_vars` annotation.
    /// If `always_mark` is true, the definition is marked as dead regardless of its liveness.
    fn incorporate_definition(
        &mut self,
        local: TempIndex,
        def: CodeOffset,
        live_vars: &LiveVarAnnotation,
    ) {
        let live_after = live_vars.get_info_at(def).after.get(&local);
        if let Some(live) = live_after {
            self.defs_alive.insert(def);
            let children = self.children.entry(def).or_default();
            live.usage_offsets().iter().for_each(|child| {
                children.insert(*child);
                self.parents.entry(*child).or_default().insert(def);
            });
            assert!(!children.is_empty(), "live var must have at least one use");
        } else {
            self.defs_dead.insert(def); // def without a use is dead
        }
    }

    /// Should `self_assign` be eliminated?
    fn should_eliminate_given_self_assign(
        self_assign_offset: CodeOffset,
        code: &[Bytecode],
        live_vars: &LiveVarAnnotation,
        eliminate_all_self_assigns: bool,
    ) -> bool {
        if !eliminate_all_self_assigns {
            // Eliminate this self assign if each of its uses are the last sources of their instructions.
            let self_assign_instr = &code[self_assign_offset as usize];
            let self_assign_temp = self_assign_instr.dests()[0];
            let live_info_after = live_vars
                .get_info_at(self_assign_offset)
                .after
                .get(&self_assign_temp);
            match live_info_after {
                None => true,
                Some(live) => live.usage_offsets().iter().all(|use_offset| {
                    let use_instr = &code[*use_offset as usize];
                    let sources = use_instr.sources();
                    sources
                        .iter()
                        .position(|source| *source == self_assign_temp)
                        .is_some_and(|pos| pos == sources.len() - 1)
                }),
            }
        } else {
            true
        }
    }
}

/// A processor which performs dead store elimination transformation.
pub struct DeadStoreElimination {
    /// If true, eliminate all self-assignments of the form `x = x`.
    /// Otherwise, only self assignments where the definition is in the same block
    /// before the self-assign are removed.
    eliminate_all_self_assigns: bool,
}

impl DeadStoreElimination {
    /// If `eliminate_all_self_assigns` is true, all self-assignments are removed.
    /// Otherwise, only self assignments where the definition is in the same block
    /// before the self-assign are removed.
    pub fn new(eliminate_all_self_assigns: bool) -> Self {
        Self {
            eliminate_all_self_assigns,
        }
    }

    /// Transforms the `code` of a function by removing the instructions corresponding to
    /// the code offsets contained in `dead_stores`.
    ///
    /// Returns the transformed code.
    fn transform(target: &FunctionTarget, dead_stores: BTreeSet<CodeOffset>) -> Vec<Bytecode> {
        let mut new_code = vec![];
        let code = target.get_bytecode();
        for (offset, instr) in code.iter().enumerate() {
            if !dead_stores.contains(&(offset as CodeOffset)) {
                new_code.push(instr.clone());
            }
        }
        new_code
    }
}

impl FunctionTargetProcessor for DeadStoreElimination {
    fn process(
        &self,
        _targets: &mut FunctionTargetsHolder,
        func_env: &FunctionEnv,
        mut data: FunctionData,
        _scc_opt: Option<&[FunctionEnv]>,
    ) -> FunctionData {
        if func_env.is_native() {
            return data;
        }
        let target = FunctionTarget::new(func_env, &data);
        let dead_stores = ReducedDefUseGraph::dead_stores(&target, self.eliminate_all_self_assigns);
        let new_code = Self::transform(&target, dead_stores);
        // Note that the file format generator will not include unused locals in the generated code,
        // so we don't need to prune unused locals here for various fields of `data` (like `local_types`).
        data.code = new_code;
        // Annotations may no longer be valid after this transformation because code offsets have changed.
        // So remove them.
        data.annotations.clear();
        data
    }

    fn name(&self) -> String {
        "DeadStoreElimination".to_string()
    }
}
