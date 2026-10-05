// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Spec functions which the inliner generates by specializing a spec
//! function for lambda arguments. A specialization is an encoding of the
//! original call; specifications printed as source restore that call, with
//! the lambdas written inline, since the specialization itself has no source.

use crate::{
    ast::{Exp, ExpData, Operation, Pattern, Spec, SpecBlockTarget},
    exp_rewriter::ExpRewriterFunctions,
    model::{GlobalEnv, NodeId, QualifiedId, SpecFunId},
    symbol::Symbol,
    ty::Type,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

/// The call of a spec function which a specialization encodes.
#[derive(Debug)]
pub struct LambdaSpecialization {
    /// The specialized spec function.
    pub original: QualifiedId<SpecFunId>,
    /// The instantiation of `original`, over the type parameters of the
    /// specialization.
    pub inst: Vec<Type>,
    /// The lambdas bound to parameters of `original`, by position. The other
    /// parameters are the leading parameters of the specialization, in order.
    pub bindings: Vec<(usize, Exp)>,
    /// The trailing parameters of the specialization, which are the free
    /// variables of the lambdas.
    pub ctx_params: Vec<Symbol>,
}

#[derive(Clone, Default)]
struct LambdaSpecializations(BTreeMap<QualifiedId<SpecFunId>, Rc<LambdaSpecialization>>);

impl GlobalEnv {
    /// Records that the spec function `qid` is a specialization of `origin`.
    pub fn add_lambda_specialization(
        &self,
        qid: QualifiedId<SpecFunId>,
        origin: LambdaSpecialization,
    ) {
        if !self.has_extension::<LambdaSpecializations>() {
            self.set_extension(LambdaSpecializations::default());
        }
        self.update_extension(|specs: &mut LambdaSpecializations| {
            specs.0.insert(qid, Rc::new(origin));
        });
    }

    /// Returns the call which the spec function `qid` encodes, if it is a
    /// specialization.
    pub fn get_lambda_specialization(
        &self,
        qid: QualifiedId<SpecFunId>,
    ) -> Option<Rc<LambdaSpecialization>> {
        self.get_extension::<LambdaSpecializations>()?
            .0
            .get(&qid)
            .cloned()
    }
}

impl ExpData {
    /// Replaces calls of specializations by the calls they encode.
    pub fn restore_lambda_calls(&self, env: &GlobalEnv) -> Exp {
        let exp = self.clone().into_exp();
        if !env.has_extension::<LambdaSpecializations>() {
            return exp;
        }
        CallRestorer { env }.rewrite_exp(exp)
    }
}

impl Spec {
    /// Replaces calls of specializations by the calls they encode.
    pub fn restore_lambda_calls(&self, env: &GlobalEnv, target: &SpecBlockTarget) -> Spec {
        if !env.has_extension::<LambdaSpecializations>() {
            return self.clone();
        }
        CallRestorer { env }.rewrite_spec_descent(target, self).1
    }
}

struct CallRestorer<'env> {
    env: &'env GlobalEnv,
}

impl ExpRewriterFunctions for CallRestorer<'_> {
    fn rewrite_call(&mut self, id: NodeId, oper: &Operation, args: &[Exp]) -> Option<Exp> {
        let Operation::SpecFunction(mid, sid, range) = oper else {
            return None;
        };
        let origin = self.env.get_lambda_specialization(mid.qualified(*sid))?;
        let num_retained = args.len().checked_sub(origin.ctx_params.len())?;
        let (retained, ctx_values) = args.split_at(num_retained);
        let call_inst = self.env.get_node_instantiation(id);
        let subst: BTreeMap<Symbol, Exp> = origin
            .ctx_params
            .iter()
            .copied()
            .zip(ctx_values.iter().cloned())
            .collect();
        let mut lambdas = BTreeMap::new();
        for (pos, lambda) in &origin.bindings {
            let lambda = lambda
                .instantiate_with_patterns(self.env, &call_inst)
                .substitute_free_vars(self.env, &subst)?;
            // A lambda can itself pass a lambda to a specialized function.
            lambdas.insert(*pos, self.rewrite_exp(lambda));
        }
        let mut retained = retained.iter().cloned();
        let new_args = (0..num_retained + lambdas.len())
            .map(|pos| lambdas.remove(&pos).or_else(|| retained.next()))
            .collect::<Option<Vec<_>>>()?;
        let new_id = self
            .env
            .new_node(self.env.get_node_loc(id), self.env.get_node_type(id));
        let inst = Type::instantiate_vec(origin.inst.clone(), &call_inst);
        if !inst.is_empty() {
            self.env.set_node_instantiation(new_id, inst);
        }
        Some(
            ExpData::Call(
                new_id,
                Operation::SpecFunction(
                    origin.original.module_id,
                    origin.original.id,
                    range.clone(),
                ),
                new_args,
            )
            .into_exp(),
        )
    }
}

impl ExpData {
    /// Substitutes the free occurrences of the given variables, renaming the
    /// binders of the expression which would capture a variable of a
    /// substituted expression. Returns `None` if a substituted variable is
    /// assigned, which has no substitute.
    fn substitute_free_vars(&self, env: &GlobalEnv, subst: &BTreeMap<Symbol, Exp>) -> Option<Exp> {
        let subst: BTreeMap<Symbol, Exp> = subst
            .iter()
            .filter(|(sym, value)| !matches!(value.as_ref(), ExpData::LocalVar(_, s) if s == *sym))
            .map(|(sym, value)| (*sym, value.clone()))
            .collect();
        let exp = self.clone().into_exp();
        if subst.is_empty() {
            return Some(exp);
        }
        let captured: BTreeSet<Symbol> =
            subst.values().flat_map(|value| value.free_vars()).collect();
        let binders = self.binder_syms();
        let pool = env.symbol_pool();
        let mut taken: BTreeSet<Symbol> = binders
            .iter()
            .chain(&captured)
            .chain(self.free_vars().iter())
            .copied()
            .collect();
        let mut renames = BTreeMap::new();
        for sym in binders.intersection(&captured) {
            let base = sym.display(pool).to_string();
            let fresh = (1..)
                .map(|count| pool.make(&format!("{}_{}", base, count)))
                .find(|candidate| !taken.contains(candidate))
                .expect("fresh symbol");
            taken.insert(fresh);
            renames.insert(*sym, fresh);
        }
        let mut substitution = Substitution {
            subst: &subst,
            renames: &renames,
            scopes: vec![],
            pending: BTreeMap::new(),
            assigns_substituted: false,
        };
        let result = substitution.rewrite_exp(exp);
        (!substitution.assigns_substituted).then_some(result)
    }
}

/// Substitutes free variables and renames the binders in `renames`. Each
/// scope maps the symbols it binds to their renamed form, or to themselves.
struct Substitution<'a> {
    subst: &'a BTreeMap<Symbol, Exp>,
    renames: &'a BTreeMap<Symbol, Symbol>,
    scopes: Vec<BTreeMap<Symbol, Symbol>>,
    /// Binders renamed by `rewrite_pattern`, by new symbol, for the scope
    /// entered next.
    pending: BTreeMap<Symbol, Symbol>,
    assigns_substituted: bool,
}

impl Substitution<'_> {
    fn bound(&self, sym: Symbol) -> Option<Symbol> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(&sym).copied())
    }
}

impl ExpRewriterFunctions for Substitution<'_> {
    fn rewrite_enter_scope<'b>(
        &mut self,
        _id: NodeId,
        vars: impl Iterator<Item = &'b (NodeId, Symbol)>,
    ) {
        let scope = vars
            .map(|(_, sym)| match self.pending.remove(sym) {
                Some(old) => (old, *sym),
                None => (*sym, *sym),
            })
            .collect();
        self.scopes.push(scope);
    }

    fn rewrite_exit_scope(&mut self, _id: NodeId) {
        self.scopes.pop();
    }

    fn rewrite_local_var(&mut self, id: NodeId, sym: Symbol) -> Option<Exp> {
        match self.bound(sym) {
            Some(new_sym) if new_sym != sym => Some(ExpData::LocalVar(id, new_sym).into_exp()),
            Some(_) => None,
            None => self.subst.get(&sym).cloned(),
        }
    }

    fn rewrite_pattern(&mut self, pat: &Pattern, creating_scope: bool) -> Option<Pattern> {
        let Pattern::Var(id, sym) = pat else {
            return None;
        };
        if creating_scope {
            let new_sym = *self.renames.get(sym)?;
            self.pending.insert(new_sym, *sym);
            return Some(Pattern::Var(*id, new_sym));
        }
        match self.bound(*sym) {
            Some(new_sym) => (new_sym != *sym).then_some(Pattern::Var(*id, new_sym)),
            None => {
                self.assigns_substituted |= self.subst.contains_key(sym);
                None
            },
        }
    }
}
