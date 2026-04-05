// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Centralized analysis of state labels in Move specification expressions.
//!
//! State labels (`S |~`, `S.. |~`, `..S |~`) annotate specification conditions
//! to reference intermediate memory states in sequential operations. This module
//! provides types and functions for classifying, querying, and normalizing these
//! labels.

use crate::{
    ast::{Exp, ExpData, MemoryLabel, MemoryRange, Operation},
    model::NodeId,
};
use std::collections::BTreeSet;

// =================================================================================================
// MemoryLabelInfo

/// Classifies labels in a set of specification conditions into three categories.
///
/// Given conditions like:
/// ```text
/// ensures ..S |~ publish<Resource>(addr, val);   // S is defined (range.post)
/// ensures S.. |~ result == result_of<f>(addr);   // S is a pre-label (range.pre)
/// aborts_if S |~ aborts_of<f>(addr);             // S is a pre-label (range.pre)
/// ```
///
/// - **defined**: labels appearing as `range.post` in ANY operation (including
///   conditional contexts). These define intermediate states.
/// - **pre_labels**: labels appearing as `range.pre` that are NOT in `defined`.
///   These reference the implicit pre-state of the enclosing context.
/// - **post_labels**: labels in `defined` that are NOT referenced as `range.pre`
///   by any operation. These define the implicit post-state of the context.
#[derive(Debug, Clone, Default)]
pub struct MemoryLabelInfo {
    pub defined: BTreeSet<MemoryLabel>,
    pub pre_labels: BTreeSet<MemoryLabel>,
    pub post_labels: BTreeSet<MemoryLabel>,
    /// Whether any condition contains a mutation operation (SpecPublish/SpecRemove/SpecUpdate),
    /// even under conditionals. When true, pre and post states may differ and `old()` is needed.
    pub has_mutations: bool,
}

impl MemoryLabelInfo {
    /// Compute label classification from a set of conditions (ensures + aborts).
    /// `entry_label` is the function's entry-state label (always a pre-label).
    pub fn from_conditions(conditions: &[&Exp], entry_label: Option<MemoryLabel>) -> Self {
        let mut defined = BTreeSet::new();
        let mut all_pre = BTreeSet::new();
        let mut has_mutations = false;

        for cond in conditions {
            // Collect all defined post-labels (from any context, including conditional)
            for label in cond.as_ref().all_defined_labels() {
                defined.insert(label);
            }
            // Collect all pre-labels from behavioral predicates and spec functions
            collect_pre_labels(cond.as_ref(), &mut all_pre);
            // Check for any mutation operations (even under conditionals)
            if !has_mutations {
                has_mutations = has_mutation_ops(cond.as_ref());
            }
        }

        // The entry label is always a pre-label (it IS the pre-state).
        if let Some(label) = entry_label {
            all_pre.insert(label);
        }

        // Three-way classification:
        // - intermediate: in both defined AND all_pre → preserved with label names
        // - pre_labels: in all_pre but NOT defined → entry state, becomes old()
        // - post_labels: in defined but NOT all_pre → exit state, stripped
        let pre_labels: BTreeSet<_> = all_pre.difference(&defined).copied().collect();
        let post_labels: BTreeSet<_> = defined.difference(&all_pre).copied().collect();

        Self {
            defined,
            pre_labels,
            post_labels,
            has_mutations,
        }
    }

    /// Labels that should be treated as "definers" for assume emission.
    pub fn definers(&self) -> &BTreeSet<MemoryLabel> {
        &self.defined
    }

    /// Intermediate labels: defined AND referenced as pre-labels.
    /// These are the meaningful intermediate states that connect sequential operations.
    pub fn intermediate_labels(&self) -> BTreeSet<MemoryLabel> {
        self.defined
            .difference(&self.post_labels)
            .copied()
            .collect()
    }

    /// Normalize labels in an expression.
    ///
    /// - `post_labels` are always stripped to `None` (implicit post-state).
    /// - `pre_labels` handling depends on `is_post_context`:
    ///   - In ensures (post) context: `Global(Some(pre_label))` → `old(Global(None))`
    ///   - In requires (pre) context: `Global(Some(pre_label))` → `Global(None)`
    /// - Intermediate labels (strictly defined AND referenced as pre) are preserved.
    pub fn normalize(
        &self,
        env: &crate::model::GlobalEnv,
        exp: &Exp,
        is_post_context: bool,
    ) -> Exp {
        struct Normalizer<'a> {
            info: &'a MemoryLabelInfo,
            env: &'a crate::model::GlobalEnv,
            is_post_context: bool,
        }

        impl crate::exp_rewriter::ExpRewriterFunctions for Normalizer<'_> {
            fn rewrite_call(&mut self, id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
                // For Global/Exists with a pre-label in post context: wrap in old()
                // Only when the function has mutations. When there are no mutations,
                // pre == post, so old() is unnecessary and would cause semantic
                // mismatches in behavioral predicates at intermediate states.
                if self.is_post_context && self.info.has_mutations {
                    match oper {
                        Operation::Global(Some(label)) if self.info.pre_labels.contains(label) => {
                            let stripped =
                                ExpData::Call(id, Operation::Global(None), args.to_vec())
                                    .into_exp();
                            let old_id = self
                                .env
                                .new_node(self.env.get_node_loc(id), self.env.get_node_type(id));
                            if let Some(inst) = self.env.get_node_instantiation_opt(id) {
                                self.env.set_node_instantiation(old_id, inst);
                            }
                            return Some(
                                ExpData::Call(old_id, Operation::Old, vec![stripped]).into_exp(),
                            );
                        },
                        Operation::Exists(Some(label)) if self.info.pre_labels.contains(label) => {
                            let stripped =
                                ExpData::Call(id, Operation::Exists(None), args.to_vec())
                                    .into_exp();
                            let old_id = self
                                .env
                                .new_node(self.env.get_node_loc(id), self.env.get_node_type(id));
                            if let Some(inst) = self.env.get_node_instantiation_opt(id) {
                                self.env.set_node_instantiation(old_id, inst);
                            }
                            return Some(
                                ExpData::Call(old_id, Operation::Old, vec![stripped]).into_exp(),
                            );
                        },
                        _ => {},
                    }
                }

                // For Global/Exists with a pre-label in pre context: just strip
                // For Global/Exists with a post-label: strip (post-state is implicit)
                match oper {
                    Operation::Global(Some(label))
                        if self.info.pre_labels.contains(label)
                            || self.info.post_labels.contains(label) =>
                    {
                        return Some(
                            ExpData::Call(id, Operation::Global(None), args.to_vec()).into_exp(),
                        );
                    },
                    Operation::Exists(Some(label))
                        if self.info.pre_labels.contains(label)
                            || self.info.post_labels.contains(label) =>
                    {
                        return Some(
                            ExpData::Call(id, Operation::Exists(None), args.to_vec()).into_exp(),
                        );
                    },
                    _ => {},
                }

                // For range-carrying operations: strip pre_labels and post_labels
                rewrite_operation_labels(id, oper, args, |label| {
                    label.filter(|l| {
                        !self.info.pre_labels.contains(l) && !self.info.post_labels.contains(l)
                    })
                })
            }
        }

        use crate::exp_rewriter::ExpRewriterFunctions;
        let mut normalizer = Normalizer {
            info: self,
            env,
            is_post_context,
        };
        normalizer.rewrite_exp(exp.clone())
    }
}

// =================================================================================================
// Standalone functions

/// Rewrite all memory labels in a range-carrying operation using a label-mapping function.
/// Returns `Some(new_exp)` if any label changed, `None` otherwise. Handles `Global`, `Exists`,
/// `Behavior`, `SpecFunction`, and `SpecPublish/Remove/Update`.
fn rewrite_operation_labels(
    id: NodeId,
    oper: &Operation,
    args: &[Exp],
    mut label_fn: impl FnMut(Option<MemoryLabel>) -> Option<MemoryLabel>,
) -> Option<Exp> {
    let mut map_range = |range: &MemoryRange| -> MemoryRange {
        MemoryRange {
            pre: label_fn(range.pre),
            post: label_fn(range.post),
        }
    };
    match oper {
        Operation::Global(label) => {
            let new_label = label_fn(*label);
            (new_label != *label)
                .then(|| ExpData::Call(id, Operation::Global(new_label), args.to_vec()).into_exp())
        },
        Operation::Exists(label) => {
            let new_label = label_fn(*label);
            (new_label != *label)
                .then(|| ExpData::Call(id, Operation::Exists(new_label), args.to_vec()).into_exp())
        },
        Operation::Behavior(kind, range) => {
            let new_range = map_range(range);
            (new_range != *range).then(|| {
                ExpData::Call(id, Operation::Behavior(*kind, new_range), args.to_vec()).into_exp()
            })
        },
        Operation::SpecFunction(mid, fid, range) => {
            let new_range = map_range(range);
            (new_range != *range).then(|| {
                ExpData::Call(
                    id,
                    Operation::SpecFunction(*mid, *fid, new_range),
                    args.to_vec(),
                )
                .into_exp()
            })
        },
        Operation::SpecPublish(range)
        | Operation::SpecRemove(range)
        | Operation::SpecUpdate(range) => {
            let new_range = map_range(range);
            (new_range != *range).then(|| {
                let new_op = match oper {
                    Operation::SpecPublish(_) => Operation::SpecPublish(new_range),
                    Operation::SpecRemove(_) => Operation::SpecRemove(new_range),
                    Operation::SpecUpdate(_) => Operation::SpecUpdate(new_range),
                    _ => unreachable!(),
                };
                ExpData::Call(id, new_op, args.to_vec()).into_exp()
            })
        },
        _ => None,
    }
}

/// Substitute a single label in an expression. Replaces `old_label` with `new_label`
/// in all positions: `Global`/`Exists` labels AND `MemoryRange` pre/post fields.
pub fn substitute_label(exp: &Exp, old_label: MemoryLabel, new_label: MemoryLabel) -> Exp {
    struct LabelSub {
        old: MemoryLabel,
        new: MemoryLabel,
    }
    impl crate::exp_rewriter::ExpRewriterFunctions for LabelSub {
        fn rewrite_call(&mut self, id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
            rewrite_operation_labels(id, oper, args, |label| {
                label.map(|l| if l == self.old { self.new } else { l })
            })
        }
    }
    use crate::exp_rewriter::ExpRewriterFunctions;
    let mut sub = LabelSub {
        old: old_label,
        new: new_label,
    };
    sub.rewrite_exp(exp.clone())
}

/// Collect all labels referenced by an expression (from any operation carrying a label).
/// This is used for dependency tracking in topological sort of label definitions.
pub fn all_labels_in_exp(exp: &Exp) -> BTreeSet<MemoryLabel> {
    let mut result = BTreeSet::new();
    exp.visit_pre_order(&mut |e| {
        if let ExpData::Call(_, op, _) = e {
            match op {
                Operation::Global(Some(l)) | Operation::Exists(Some(l)) => {
                    result.insert(*l);
                },
                Operation::Behavior(_, r)
                | Operation::SpecFunction(_, _, r)
                | Operation::SpecPublish(r)
                | Operation::SpecRemove(r)
                | Operation::SpecUpdate(r) => {
                    for l in r.labels() {
                        result.insert(l);
                    }
                },
                _ => {},
            }
        }
        true
    });
    result
}

// =================================================================================================
// Helpers

/// Collect all pre-state labels from an expression. These are:
/// - `range.pre` labels from `Behavior` and `SpecFunction` operations
/// - Labels on `Global`/`Exists` that appear inside a mutation builtin's
///   value argument (`SpecUpdate`/`SpecPublish` arg[1]). These reference
///   the pre-state memory by construction (the mutation transforms pre→post).
fn collect_pre_labels(exp: &ExpData, labels: &mut BTreeSet<MemoryLabel>) {
    exp.visit_pre_order(&mut |e| {
        if let ExpData::Call(_, op, _) = e {
            match op {
                Operation::Behavior(_, range)
                | Operation::SpecFunction(_, _, range)
                | Operation::SpecPublish(range)
                | Operation::SpecRemove(range)
                | Operation::SpecUpdate(range) => {
                    if let Some(label) = range.pre {
                        labels.insert(label);
                    }
                },
                _ => {},
            }
        }
        true
    });
    // Separately collect Global/Exists labels inside mutation value args
    collect_mutation_value_labels(exp, labels);
}

/// Walk expression tree looking for mutation builtins. When found, collect
/// Global/Exists labels from their value argument subtree.
fn collect_mutation_value_labels(exp: &ExpData, labels: &mut BTreeSet<MemoryLabel>) {
    exp.visit_pre_order(&mut |e| {
        if let ExpData::Call(_, op, args) = e {
            match op {
                Operation::SpecUpdate(_) | Operation::SpecPublish(_) => {
                    // args[1] = value (references pre-state memory)
                    if let Some(val) = args.get(1) {
                        collect_globals_in_subtree(val.as_ref(), labels);
                    }
                },
                _ => {},
            }
        }
        true
    });
}

/// Check if an expression is a tautological WellFormed condition.
/// Matches `WellFormed(x)` and `forall x: Domain(): WellFormed(x)`.
fn is_wellformed_tautology(exp: &ExpData) -> bool {
    match exp {
        ExpData::Call(_, Operation::WellFormed, _) => true,
        ExpData::Quant(_, crate::ast::QuantKind::Forall, _, _, where_clause, body) => {
            where_clause.is_none() && is_wellformed_tautology(body.as_ref())
        },
        _ => false,
    }
}

/// Check if an expression contains any mutation operations (SpecPublish/SpecRemove/SpecUpdate),
/// even under conditionals. This detects functions that may modify state.
fn has_mutation_ops(exp: &ExpData) -> bool {
    let mut found = false;
    exp.visit_pre_order(&mut |e| {
        if !found {
            if let ExpData::Call(_, op, _) = e {
                if matches!(
                    op,
                    Operation::SpecPublish(_) | Operation::SpecRemove(_) | Operation::SpecUpdate(_)
                ) {
                    found = true;
                }
            }
        }
        !found
    });
    found
}

/// Collect all Global/Exists labels in a subtree (inside mutation value context).
fn collect_globals_in_subtree(exp: &ExpData, labels: &mut BTreeSet<MemoryLabel>) {
    exp.visit_pre_order(&mut |e| {
        if let ExpData::Call(_, op, _) = e {
            match op {
                Operation::Global(Some(label)) | Operation::Exists(Some(label)) => {
                    labels.insert(*label);
                },
                _ => {},
            }
        }
        true
    });
}

// =================================================================================================
// ExpData methods for strict label analysis

impl ExpData {
    /// Collects ALL memory labels defined as `range.post` in any operation,
    /// regardless of whether they appear under conditionals.
    pub fn all_defined_labels(&self) -> BTreeSet<MemoryLabel> {
        let mut result = BTreeSet::new();
        self.visit_pre_order(&mut |e| {
            if let ExpData::Call(_, op, _) = e {
                let range = match op {
                    Operation::Behavior(_, r)
                    | Operation::SpecFunction(_, _, r)
                    | Operation::SpecPublish(r)
                    | Operation::SpecRemove(r)
                    | Operation::SpecUpdate(r) => Some(r),
                    _ => None,
                };
                if let Some(MemoryRange {
                    post: Some(label), ..
                }) = range
                {
                    result.insert(*label);
                }
            }
            true
        });
        result
    }

    /// Collects memory labels that are defined by strictly-evaluated sub-expressions.
    /// A label is "strictly defined" if it appears as `range.post` in a `Behavior`,
    /// `SpecFunction`, `SpecPublish`, `SpecRemove`, or `SpecUpdate` operation that is
    /// NOT guarded by a conditional (`IfElse`, `Implies`, `Iff`, `Match`).
    /// Only strictly-evaluated definitions guarantee the Boogie axiom fires,
    /// constraining the labeled memory variable.
    pub fn strictly_defined_labels(&self) -> BTreeSet<MemoryLabel> {
        let mut result = BTreeSet::new();
        self.collect_strict_labels(true, &mut result);
        result
    }

    fn collect_strict_labels(&self, strict: bool, labels: &mut BTreeSet<MemoryLabel>) {
        use ExpData::*;
        use Operation::*;
        match self {
            Call(_, op, args) => {
                if strict {
                    let range = match op {
                        Behavior(_, r)
                        | SpecFunction(_, _, r)
                        | SpecPublish(r)
                        | SpecRemove(r)
                        | SpecUpdate(r) => Some(r),
                        _ => None,
                    };
                    if let Some(MemoryRange {
                        post: Some(label), ..
                    }) = range
                    {
                        labels.insert(*label);
                    }
                }
                match op {
                    Implies => {
                        // RHS is conditional UNLESS the LHS is a tautology
                        // (e.g., WellFormed assumptions that are always true).
                        if let [lhs, rhs] = args.as_slice() {
                            let lhs_is_tautology = is_wellformed_tautology(lhs.as_ref());
                            lhs.as_ref().collect_strict_labels(strict, labels);
                            rhs.as_ref().collect_strict_labels(
                                if lhs_is_tautology { strict } else { false },
                                labels,
                            );
                        }
                    },
                    Iff | Or => {
                        // Both sides conditional
                        for arg in args {
                            arg.as_ref().collect_strict_labels(false, labels);
                        }
                    },
                    _ => {
                        for arg in args {
                            arg.as_ref().collect_strict_labels(strict, labels);
                        }
                    },
                }
            },
            IfElse(_, cond, then_exp, else_exp) => {
                cond.as_ref().collect_strict_labels(strict, labels);
                then_exp.as_ref().collect_strict_labels(false, labels);
                else_exp.as_ref().collect_strict_labels(false, labels);
            },
            Match(_, matched, arms) => {
                matched.as_ref().collect_strict_labels(strict, labels);
                for arm in arms {
                    if let Some(cond) = &arm.condition {
                        cond.as_ref().collect_strict_labels(false, labels);
                    }
                    arm.body.as_ref().collect_strict_labels(false, labels);
                }
            },
            Block(_, _, binding, body) => {
                if let Some(b) = binding {
                    b.as_ref().collect_strict_labels(strict, labels);
                }
                body.as_ref().collect_strict_labels(strict, labels);
            },
            Sequence(_, exps) => {
                for e in exps {
                    e.as_ref().collect_strict_labels(strict, labels);
                }
            },
            Quant(_, _, _, _, condition, body) => {
                if let Some(c) = condition {
                    c.as_ref().collect_strict_labels(false, labels);
                }
                body.as_ref().collect_strict_labels(strict, labels);
            },
            _ => {},
        }
    }
}
