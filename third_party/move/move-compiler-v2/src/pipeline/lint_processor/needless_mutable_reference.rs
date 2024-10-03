// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements a stackless-bytecode linter that checks for mutable references
//! that are never used mutably, and suggests to use immutable references instead.
//! For example, if a mutable reference is never written to or passed as a mutable reference
//! parameter to a function call, it can be replaced with an immutable reference.
//!
//! Currently, we only track mutable references that are:
//! - function parameters,
//! - obtained via `&mut` or `borrow_global_mut`.

use crate::{lint_common::LintChecker, pipeline::lint_processor::StacklessBytecodeLinter};
use move_model::{
    ast::TempIndex,
    model::{GlobalEnv, Loc, Parameter},
};
use move_stackless_bytecode::{
    function_target::FunctionTarget,
    stackless_bytecode::{Bytecode, Operation},
};
use std::collections::{BTreeMap, BTreeSet};

/// Track "mutable" usages of certain mutable references in a function.
/// Currently, the tracking is performed conservatively, in a flow-insensitive
/// manner, to minimize perceived false positives.
#[derive(Default)]
struct MutableReferenceUsageTracker {
    /// Keys are temps which are origins of certain mutable references.
    /// Each key is mapped to a location where it acquires the mutable reference.
    /// The origins tracked currently are:
    /// - function parameters that are mutable references.
    /// - mutable references acquired through `&mut` or `borrow_global_mut`.
    /// The list above can be extended in the future.
    origins: BTreeMap<TempIndex, Loc>,
    /// Derived edges from mutable references.
    /// A derived edge y -> x is created in the following cases:
    /// - `x = y;`,  where y: &mut
    /// - `x = &mut y.f;`
    /// Each origin also has an entry in `derived_edges` (usually mapping to an
    /// empty set, unless an origin is also derived).
    derived_edges: BTreeMap<TempIndex, BTreeSet<TempIndex>>,
    /// The set of mutable references that are known to be used mutably, either directly
    /// or through a derived edge. To be used mutably, the mutable reference is:
    /// - written to via `WriteRef`, or
    /// - passed as an argument to a function call's mutable reference parameter.
    mutably_used: BTreeSet<TempIndex>,
}

impl MutableReferenceUsageTracker {
    /// For the `target` function, get locations we can warn about needless mutable references.
    pub fn get_needless_mutable_refs(target: &FunctionTarget) -> Vec<Loc> {
        let mut tracker = Self::get_tracker_from_params(target);
        for instr in target.get_bytecode() {
            tracker.update(target, instr);
        }
        tracker.get_mutably_unused_locations()
    }

    /// Get an initial tracker from function parameters.
    fn get_tracker_from_params(target: &FunctionTarget) -> Self {
        let mut tracker = Self::default();
        for (origin, loc) in Self::get_mut_reference_params(target) {
            tracker.add_origin(origin, loc);
        }
        tracker
    }

    /// Get locations where origins are not used mutably.
    fn get_mutably_unused_locations(self) -> Vec<Loc> {
        self.origins
            .into_iter()
            .filter_map(|(t, loc)| {
                if !self.mutably_used.contains(&t) {
                    Some(loc)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get mutable reference function parameters, along with their location info.
    fn get_mut_reference_params(target: &FunctionTarget) -> BTreeMap<TempIndex, Loc> {
        target
            .func_env
            .get_parameters_ref()
            .iter()
            .enumerate()
            .filter_map(|(i, Parameter(_, ty, loc))| {
                if ty.is_mutable_reference() {
                    // Note: we assume that parameters are laid out as the initial temps.
                    Some((i, loc.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Update the tracker given `instr`.
    fn update(&mut self, target: &FunctionTarget, instr: &Bytecode) {
        self.update_origins(target, instr);
        self.update_derived_edges(target, instr);
        self.update_mutably_used(target.global_env(), instr);
    }

    /// Update origins given `instr`.
    fn update_origins(&mut self, target: &FunctionTarget, instr: &Bytecode) {
        use Bytecode::*;
        use Operation::*;
        // We currently track `&mut` and `borrow_global_mut` as origins.
        if let Call(id, dsts, BorrowLoc | BorrowGlobal(..), _, _) = instr {
            debug_assert!(dsts.len() == 1);
            if target.get_local_type(dsts[0]).is_mutable_reference() {
                // The location of whichever instruction appears first for the origin
                // is used for reporting.
                self.add_origin(dsts[0], target.get_bytecode_loc(*id));
            }
        }
    }

    /// Update derived edges given `instr`.
    fn update_derived_edges(&mut self, target: &FunctionTarget, instr: &Bytecode) {
        use Bytecode::*;
        use Operation::*;
        match instr {
            Assign(_, dst, src, _) => {
                if self.node_exists(*src) {
                    self.add_derived_edge(*dst, *src);
                }
            },
            Call(_, dsts, BorrowField(..) | BorrowVariantField(..), srcs, ..) => {
                debug_assert!(srcs.len() == 1 && dsts.len() == 1);
                if self.node_exists(srcs[0])
                    && target.get_local_type(dsts[0]).is_mutable_reference()
                {
                    self.add_derived_edge(dsts[0], srcs[0]);
                }
            },
            _ => {},
        }
    }

    /// Update mutable usages given `instr`.
    fn update_mutably_used(&mut self, env: &GlobalEnv, instr: &Bytecode) {
        use Bytecode::*;
        use Operation::*;
        match instr {
            Call(_, _, WriteRef, srcs, _) => {
                self.set_and_propagate_mutably_used(srcs[0]);
            },
            Call(_, _, Function(mid, fid, _), srcs, _) => {
                let callee_env = env.get_function_qid(mid.qualified(*fid));
                callee_env
                    .get_parameter_types()
                    .iter()
                    .enumerate()
                    .filter(|(_, ty)| ty.is_mutable_reference())
                    .map(|(i, _)| srcs[i])
                    .for_each(|src| self.set_and_propagate_mutably_used(src));
            },
            _ => {},
        }
    }

    /// Add an origin to the tracker.
    fn add_origin(&mut self, origin: TempIndex, loc: Loc) {
        self.origins.entry(origin).or_insert(loc);
        self.derived_edges.entry(origin).or_default();
    }

    /// Check if a node exists in the tracker.
    fn node_exists(&self, node: TempIndex) -> bool {
        self.derived_edges.contains_key(&node)
    }

    /// Add a derived edge to the tracker.
    fn add_derived_edge(&mut self, from: TempIndex, to: TempIndex) {
        self.derived_edges.entry(from).or_default().insert(to);
        self.propagate_mutably_used(from, to);
    }

    /// Propagate mutably used information from `from` to `to`.
    fn propagate_mutably_used(&mut self, from: TempIndex, to: TempIndex) {
        if self.mutably_used.contains(&from) {
            self.set_and_propagate_mutably_used(to);
        }
    }

    /// Set a mutable reference as mutably used.
    /// Propagate this information transitively through derived edges.
    /// Propagation is stopped early if a node is already marked as mutably used.
    fn set_and_propagate_mutably_used(&mut self, node: TempIndex) {
        let mut mutably_used = std::mem::take(&mut self.mutably_used);
        self.set_and_propagate_mutably_used_helper(node, &mut mutably_used);
        self.mutably_used = mutably_used;
    }

    /// Helper function for `set_and_propagate_mutably_used`.
    /// Note that `self` is lacking the `mutably_used` field for the duration of
    /// this method (and it is instead passed separately and explicitly).
    fn set_and_propagate_mutably_used_helper(
        &self,
        node: TempIndex,
        mutably_used: &mut BTreeSet<TempIndex>,
    ) {
        if !mutably_used.insert(node) {
            // Stop early if a node is already marked as mutably used.
            return;
        }
        if let Some(parents) = self.derived_edges.get(&node) {
            for parent in parents {
                self.set_and_propagate_mutably_used_helper(*parent, mutably_used);
            }
        }
    }
}

/// Linter for detecting needless mutable references.
pub struct NeedlessMutableReference {}

impl StacklessBytecodeLinter for NeedlessMutableReference {
    fn get_lint_checker(&self) -> LintChecker {
        LintChecker::NeedlessMutableReference
    }

    fn check(&self, target: &FunctionTarget) {
        let needless_mutable_refs = MutableReferenceUsageTracker::get_needless_mutable_refs(target);
        for loc in needless_mutable_refs {
            if loc.is_inlined() {
                continue;
            }
            self.warning(
                target.global_env(),
                &loc,
                "Needless mutable reference or borrow: consider using immutable reference or borrow instead",
            );
        }
    }
}
