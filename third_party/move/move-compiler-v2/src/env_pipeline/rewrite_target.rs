// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_model::{
    ast::{Exp, Spec, SpecBlockTarget},
    model::{FunId, GlobalEnv, NodeId, QualifiedId, SpecFunId},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    mem,
};

/// Represents a target for rewriting.
#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Clone)]
pub enum RewriteTarget {
    /// A Move function
    MoveFun(QualifiedId<FunId>),
    /// A specification function
    SpecFun(QualifiedId<SpecFunId>),
    /// A specification block, which can be attached to a Move function, struct, or module.
    SpecBlock(SpecBlockTarget),
}

/// Represents the state of a rewriting target.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RewriteState {
    /// The target has not been changed
    Unchanged,
    /// The definition of a Move or spec function has changed
    Def(Exp),
    /// A specification block has changed.
    Spec(Spec),
    /// The target is 'abstract', i.e. does have a definition.
    Abstract,
}

/// Scope for collecting targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum RewritingScope {
    /// Including only targets for which `module.is_target()` is true.
    CompilationTarget,
    /// Include everything.
    Everything,
}

/// Represents a set of rewriting targets in the given state.
#[derive(Clone)]
pub struct RewriteTargets {
    pub targets: BTreeMap<RewriteTarget, RewriteState>,
}

impl RewriteTargets {
    /// Create a new set of rewrite targets, collecting them as specified by `scope`.
    /// Those targets are initially associated with `Unchanged` state.
    pub fn create(env: &GlobalEnv, scope: RewritingScope) -> Self {
        let mut targets = vec![];
        let add_spec =
            |targets: &mut Vec<RewriteTarget>, sb_target: SpecBlockTarget, spec: &Spec| {
                if !spec.is_empty() {
                    targets.push(RewriteTarget::SpecBlock(sb_target))
                }
            };
        for module in env.get_modules() {
            if scope == RewritingScope::Everything || module.is_target() {
                for func in module.get_functions() {
                    let id = func.get_qualified_id();
                    targets.push(RewriteTarget::MoveFun(id));
                    add_spec(
                        &mut targets,
                        SpecBlockTarget::Function(id.module_id, id.id),
                        &func.get_spec(),
                    );
                }
                for (spec_fun_id, _) in module.get_spec_funs() {
                    targets.push(RewriteTarget::SpecFun(
                        module.get_id().qualified(*spec_fun_id),
                    ));
                }
                for struct_env in module.get_structs() {
                    add_spec(
                        &mut targets,
                        SpecBlockTarget::Struct(module.get_id(), struct_env.get_id()),
                        &struct_env.get_spec(),
                    )
                }
                if !module.get_spec().is_empty() {
                    add_spec(
                        &mut targets,
                        SpecBlockTarget::Module(module.get_id()),
                        &module.get_spec(),
                    );
                }
            }
        }
        Self {
            targets: targets
                .into_iter()
                .map(|target| (target, RewriteState::Unchanged))
                .collect(),
        }
    }

    /// Filters the targets according to the predicate.
    pub fn filter(&mut self, pred: impl Fn(&RewriteTarget, &RewriteState) -> bool) {
        self.targets = mem::take(&mut self.targets)
            .into_iter()
            .filter(|(t, s)| pred(t, s))
            .collect();
    }

    /// Iterates all targets.
    pub fn iter(&self) -> impl Iterator<Item = (&RewriteTarget, &RewriteState)> + '_ {
        self.targets.iter()
    }

    /// Returns an iteration of the target keys.
    pub fn keys(&self) -> impl Iterator<Item = RewriteTarget> + '_ {
        self.targets.keys().cloned()
    }

    /// Adds a new rewrite target in state `Unchanged` if it doesn't exist yet. Returns
    /// a boolean whether the entry is new and a mutable reference to the state.
    pub fn entry(&mut self, target: RewriteTarget) -> (bool, &mut RewriteState) {
        let mut is_new = false;
        let state = self.targets.entry(target).or_insert_with(|| {
            is_new = true;
            RewriteState::Unchanged
        });
        (is_new, state)
    }

    /// Gets the current state of the target.
    pub fn state(&self, target: &RewriteTarget) -> &RewriteState {
        self.targets.get(target).expect("state defined")
    }

    /// Gets the mutable current state of the target.
    pub fn state_mut(&mut self, target: &RewriteTarget) -> &mut RewriteState {
        self.targets.get_mut(target).expect("state defined")
    }

    /// Updates the global env based on the current state. This consumes
    /// the rewrite targets.
    pub fn write_to_env(self, env: &mut GlobalEnv) {
        for (target, state) in self.targets {
            use RewriteState::*;
            use RewriteTarget::*;
            match (target, state) {
                (_, Unchanged) => {},
                (MoveFun(fnid), Def(def)) => env.set_function_def(fnid, def),
                (SpecFun(fnid), Def(def)) => env.get_spec_fun_mut(fnid).body = Some(def),
                (SpecBlock(sb_target), Spec(spec)) => {
                    *env.get_spec_block_mut(&sb_target) = spec;
                },
                _ => panic!("unexpected rewrite target and result combination"),
            }
        }
    }
}

impl RewriteTarget {
    /// Gets the functions called or referenced in the target.
    pub fn used_funs_with_uses(
        &self,
        env: &GlobalEnv,
    ) -> BTreeMap<QualifiedId<FunId>, BTreeSet<NodeId>> {
        use RewriteTarget::*;
        match self {
            MoveFun(id) => env
                .get_function(*id)
                .get_def()
                .map(|e| e.called_funs_with_callsites())
                .unwrap_or_default(),
            SpecFun(id) => env
                .get_spec_fun(*id)
                .body
                .as_ref()
                .map(|e| e.called_funs_with_callsites())
                .unwrap_or_default(),
            SpecBlock(target) => {
                let spec = env.get_spec_block(target);
                spec.used_funs_with_uses()
            },
        }
    }

    /// Get the environment state of this target in form of a RewriteState.
    pub fn get_env_state(&self, env: &GlobalEnv) -> RewriteState {
        use RewriteState::*;
        use RewriteTarget::*;
        match self {
            MoveFun(fid) => env
                .get_function(*fid)
                .get_def()
                .map(|e| Def(e.clone()))
                .unwrap_or(Abstract),
            SpecFun(fid) => env
                .get_spec_fun(*fid)
                .body
                .clone()
                .map(Def)
                .unwrap_or(Abstract),
            SpecBlock(sb_target) => Spec(env.get_spec_block(sb_target).clone()),
        }
    }
}
