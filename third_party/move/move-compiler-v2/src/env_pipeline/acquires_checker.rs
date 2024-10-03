// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Performs strict acquires analysis as outlined in the move book, that is:
//! A Move function `m::f` must be annotated with acquires `T`` if and only if,
//! - The body of `m::f` contains a `move_from<T>`, `borrow_global_mut<T>`, or `borrow_global<T>` instruction, or
//! - The body of `m::f` invokes a function `m::g` declared in the same module that is annotated with acquires
//! Warn if access specifiers other than plain `acquires R` is used.
//! This check is enabled by flag `Experiment::ACQUIRES_CHECK`.

use move_model::{
    ast::{ExpData, Operation, ResourceSpecifier, VisitorPosition},
    model::{FunId, FunctionEnv, GlobalEnv, Loc, ModuleEnv, StructId},
    ty::Type,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Performs acquires checking
pub fn acquires_checker(env: &GlobalEnv) {
    for module in env.get_modules() {
        if module.is_target() {
            let analyzer = AcquireChecker::new(module.clone());
            let acquires = analyzer.analyze();
            for (fun_id, acquires) in acquires.into_iter() {
                let fun_env = module.get_function(fun_id);
                if fun_env.is_inline() {
                    continue;
                }
                let mut declared_acquires = get_acquired_resources(&fun_env);
                for (sid, acquired) in acquires.0 {
                    if declared_acquires.remove(&sid).is_none() {
                        let s_name = module.get_struct(sid).get_name();
                        let note = match acquired {
                            AcquiredAt::Directly(loc) => (loc, "acquired here".to_owned()),
                            AcquiredAt::Indirectly(loc, callee_id) => (
                                loc,
                                format!(
                                    "acquired by the call to `{}`",
                                    module
                                        .get_function(callee_id)
                                        .get_name()
                                        .display(module.symbol_pool())
                                ),
                            ),
                        };
                        env.error_with_labels(
                            &fun_env.get_id_loc(),
                            &format!(
                                "missing acquires annotation for `{}`",
                                s_name.display(env.symbol_pool())
                            ),
                            vec![note],
                        )
                    }
                }
                for (_sid, loc) in declared_acquires {
                    env.error(&loc, "unnecessary acquires annotation");
                }
            }
        }
    }
}

/// Gets the acquired resources declared by `acquires R`
fn get_acquired_resources(fun_env: &FunctionEnv) -> BTreeMap<StructId, Loc> {
    if let Some(access_specifiers) = fun_env.get_access_specifiers() {
        access_specifiers
            .iter()
            .filter_map(|access_specifier| {
                if let ResourceSpecifier::Resource(inst_qid) = &access_specifier.resource.1 {
                    if inst_qid.module_id != fun_env.module_env.get_id() {
                        fun_env.module_env.env.error(
                            &access_specifier.resource.0,
                            "acquires a resource from another module",
                        )
                    }
                    Some((inst_qid.id, access_specifier.resource.0.clone()))
                } else {
                    None
                }
            })
            .collect()
    } else {
        BTreeMap::new()
    }
}

#[derive(Debug)]
enum AcquiredAt {
    /// Acquired by move_from<T>, borrow_global_mut<T>, or borrow_global<T>
    Directly(Loc),
    /// Acquired by a call to another function
    Indirectly(Loc, FunId),
}

impl AcquiredAt {
    #[allow(unused)]
    fn get_loc(&self) -> &Loc {
        match self {
            AcquiredAt::Directly(loc) => loc,
            AcquiredAt::Indirectly(loc, _) => loc,
        }
    }
}

/// Maps to resource acquired by where it's acquired
#[derive(Debug)]
struct AcquiredResources(BTreeMap<StructId, AcquiredAt>);

impl AcquiredResources {
    /// Joins the resources acquired by `other_fun`
    fn join(&mut self, other_fun: FunId, other_fun_called_at: Loc, other_acquries: &Self) -> bool {
        let mut changed = false;
        for (sid, _) in other_acquries.0.iter() {
            use std::collections::btree_map::Entry::*;
            match self.0.entry(*sid) {
                Vacant(e) => {
                    e.insert(AcquiredAt::Indirectly(
                        other_fun_called_at.clone(),
                        other_fun,
                    ));
                    changed = true;
                },
                Occupied(_) => {},
            }
        }
        changed
    }
}

struct AcquireChecker<'a> {
    mod_env: ModuleEnv<'a>,
}

impl<'a> AcquireChecker<'a> {
    pub fn new(mod_env: ModuleEnv<'a>) -> Self {
        Self { mod_env }
    }

    /// Computes the resources acquired by each function in the module
    pub fn analyze(&self) -> BTreeMap<FunId, AcquiredResources> {
        let (call_graph, acquire_env) = self.get_call_graph_and_directly_acquired_resources();
        self.compute_fixed_point(call_graph, acquire_env)
    }

    /// Returns
    /// - the call graph where `f` maps to `(g, loc)` if `f` calls `g` at `loc`,
    /// (just one example is provided if there are multiple such calls)
    /// only functions defined in the current module are included
    /// - a map from functions to resources directly acquired by `move_from<T>`, `borrow_global_mut<T>`, or `borrow_global<T>`
    /// by the function
    fn get_call_graph_and_directly_acquired_resources(
        &self,
    ) -> (
        BTreeMap<FunId, BTreeMap<FunId, Loc>>,
        BTreeMap<FunId, AcquiredResources>,
    ) {
        let mut call_graph = BTreeMap::new();
        let mut acquire_env = BTreeMap::new();
        for fun_env in self.mod_env.get_functions() {
            let fun_id = fun_env.get_id();
            let (callees, resources) = get_callees_and_acquired_resources(fun_env);
            call_graph.insert(fun_id, callees);
            let acquired = resources
                .into_iter()
                .map(|(sid, loc)| (sid, AcquiredAt::Directly(loc)))
                .collect();
            acquire_env.insert(fun_id, AcquiredResources(acquired));
        }
        (call_graph, acquire_env)
    }

    fn compute_fixed_point(
        &self,
        call_graph: BTreeMap<FunId, BTreeMap<FunId, Loc>>,
        mut acquire_env: BTreeMap<FunId, AcquiredResources>,
    ) -> BTreeMap<FunId, AcquiredResources> {
        let reversed_call_graph = call_graph.iter().fold(
            BTreeMap::new(),
            |mut reversed: BTreeMap<FunId, BTreeSet<FunId>>, (caller, callees)| {
                for (callee, _) in callees.iter() {
                    reversed.entry(*callee).or_default().insert(*caller);
                }
                reversed
            },
        );
        let mut work_list: VecDeque<_> = self
            .mod_env
            .get_functions()
            .map(|fun_env| fun_env.get_id())
            .collect();
        while let Some(fun_id) = work_list.pop_front() {
            let mut any_changes = false;
            let mut caller_acquires = acquire_env.remove(&fun_id).expect("acquired resources");
            for (callee, loc) in call_graph.get(&fun_id).expect("callees") {
                if *callee == fun_id {
                    continue;
                }
                let callee_acquires = acquire_env.get(callee).expect("callee acquires");
                let changed = caller_acquires.join(*callee, loc.clone(), callee_acquires);
                any_changes = any_changes || changed;
            }
            acquire_env.insert(fun_id, caller_acquires);
            if any_changes {
                if let Some(callers) = reversed_call_graph.get(&fun_id) {
                    for caller in callers {
                        work_list.push_back(*caller);
                    }
                }
            }
        }
        acquire_env
    }
}

/// Suppose the given function is defined in module M.
/// Returns
/// - the calles of the given function that are defined in M
/// - resources acquired by move_from\<T>, borrow_global_mut\<T>, or borrow_global\<T>,
/// where T is define in M
fn get_callees_and_acquired_resources(
    fun_env: FunctionEnv,
) -> (BTreeMap<FunId, Loc>, BTreeMap<StructId, Loc>) {
    let mut callees = BTreeMap::new();
    let mut resources = BTreeMap::new();
    let mid = fun_env.module_env.get_id();
    if let Some(fun_body) = fun_env.get_def() {
        let mut in_spec_block = false;
        let mut collect_callees = |pos, exp: &ExpData| match exp {
            ExpData::Call(node_id, op, _) => {
                if !in_spec_block && matches!(pos, VisitorPosition::Pre) {
                    if let Operation::MoveFunction(exp_mid, exp_fid) = op {
                        if *exp_mid == fun_env.module_env.get_id() {
                            let loc = fun_env.module_env.env.get_node_loc(*node_id);
                            callees.entry(*exp_fid).or_insert(loc);
                        }
                    }
                }
                true
            },
            ExpData::SpecBlock(..) => {
                match pos {
                    VisitorPosition::Pre => in_spec_block = true,
                    VisitorPosition::Post => in_spec_block = false,
                    _ => {},
                }
                true
            },
            _ => true,
        };
        let mut in_spec_block = false;
        let mut collect_directly_used_resources = |pos, exp: &ExpData| match exp {
            ExpData::Call(node_id, op, _) => {
                if matches!(pos, VisitorPosition::Pre) {
                    match op {
                        Operation::MoveFrom | Operation::BorrowGlobal(..) => {
                            let ty_params = fun_env.module_env.env.get_node_instantiation(*node_id);
                            let ty_param = ty_params.first().expect("type parameter");
                            if let Type::Struct(exp_mid, sid, _insts) = ty_param {
                                if *exp_mid == mid {
                                    let loc = fun_env.module_env.env.get_node_loc(*node_id);
                                    resources.entry(*sid).or_insert(loc);
                                }
                            }
                        },
                        _ => {},
                    }
                }
                true
            },
            ExpData::SpecBlock(..) => {
                match pos {
                    VisitorPosition::Pre => in_spec_block = true,
                    VisitorPosition::Post => in_spec_block = false,
                    _ => {},
                }
                true
            },
            _ => true,
        };
        fun_body.visit_positions(&mut |pos, e| {
            collect_callees(pos.clone(), e) && collect_directly_used_resources(pos, e)
        });
    }
    (callees, resources)
}
