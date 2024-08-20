// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use itertools::Itertools;
use move_model::{
    ast::{ExpData, Operation, VisitorPosition},
    model::{FunId, GlobalEnv, Loc, ModuleEnv, NodeId, QualifiedId, QualifiedInstId},
    ty::Type,
};

/// Checks all modules in `env`
pub fn check_cyclic_instantiations(env: &GlobalEnv) {
    for module in env.get_modules() {
        if module.is_target() {
            let checker = CyclicInstantiationChecker::new(module);
            checker.check();
        }
    }
}

/// Module checker state
struct CyclicInstantiationChecker<'a> {
    /// The module we are checking
    mod_env: ModuleEnv<'a>,
}

impl<'a> CyclicInstantiationChecker<'a> {
    pub fn new(mod_env: ModuleEnv<'a>) -> Self {
        Self { mod_env }
    }

    /// Checks all functions in the module
    fn check(&self) {
        for fun_env in self.mod_env.get_functions() {
            self.check_fun(fun_env.get_id())
        }
    }

    /// Checks the given function
    fn check_fun(&self, fun_id: FunId) {
        let fun_env = self.mod_env.get_function(fun_id);
        if let Some(fun_body) = fun_env.get_def() {
            let mut callers = self.gen_init_callers_chain(fun_id);
            let insts = self.gen_generic_insts_for_fun(fun_id);
            fun_body.visit_positions(&mut |pos, e| self.visit(pos, e, &insts, &mut callers));
        }
    }

    /// Generates generic type instantiations for the given function
    fn gen_generic_insts_for_fun(&self, fun_id: FunId) -> Vec<Type> {
        let num_ty_params = self.mod_env.get_function(fun_id).get_type_parameter_count() as u16;
        (0..num_ty_params).map(Type::TypeParameter).collect_vec()
    }

    /// Generates the initial callers chain for the given function,
    /// which is the given function initialized with generic type parameters
    fn gen_init_callers_chain(&self, fun_id: FunId) -> Vec<(Loc, QualifiedInstId<FunId>)> {
        let insts = self.gen_generic_insts_for_fun(fun_id);
        let root_caller = self.mod_env.get_id().qualified_inst(fun_id, insts);
        let root_caller_loc = self.mod_env.get_function(fun_id).get_loc();
        vec![(root_caller_loc, root_caller)]
    }

    /// Visits an expression and checks for cyclic type instantiations.
    /// `insts`: the type parameters of the current expression
    /// `callers_chain`: the chain of callers leading to the current expression
    fn visit(
        &self,
        position: VisitorPosition,
        e: &ExpData,
        insts: &[Type],
        callers_chain: &mut Vec<(Loc, QualifiedInstId<FunId>)>,
    ) -> bool {
        use ExpData::*;
        use VisitorPosition::*;
        match (position, e) {
            (Pre, Call(nid, op, _)) => self.visit_call(nid, op, insts, callers_chain),
            _ => true,
        }
    }

    /// Visits a call expression and checks for cyclic type instantiations.
    /// Other parameters are the same as in `visit`
    fn visit_call(
        &self,
        nid: &NodeId,
        op: &Operation,
        insts: &[Type],
        callers_chain: &mut Vec<(Loc, QualifiedInstId<FunId>)>,
    ) -> bool {
        if let Operation::MoveFunction(mod_id, fun_id) = op {
            let callee_uninst = mod_id.qualified_inst(*fun_id, self.get_inst(*nid));
            let callee = callee_uninst.instantiate(insts);
            if *mod_id != self.mod_env.get_id() || self.def_not_recursive(callee.to_qualified_id())
            {
                // skips if callee from another module (since there is no cyclic module dependency),
                // or if the callee is not recursive
                true
            } else {
                for (_, ancester_caller) in callers_chain.iter() {
                    if ancester_caller.to_qualified_id() == callee.to_qualified_id() {
                        // we are checking for the root caller
                        let (_, checking_for) = &callers_chain[0];
                        if checking_for.to_qualified_id() != callee.to_qualified_id() {
                            // check and report diagnostics when `callee` is checked
                            // this happens when root caller `f` calls `g` which then calls `g` itself
                            return true;
                        } else {
                            #[allow(clippy::collapsible_else_if)]
                            if let Some(_ty_param) = callee
                                .inst
                                .iter()
                                .filter_map(ty_properly_contains_ty_parameter)
                                .next()
                            {
                                self.report_error(*nid, callee, callers_chain);
                                return false;
                            } else {
                                return true;
                            }
                        }
                    }
                }
                self.visit_callees(*nid, callee, insts, callers_chain)
            }
        } else {
            true
        }
    }

    /// Visits a call expression and checks for cyclic type instantiations.
    /// Other parameters are the same as in `visit`.
    /// Precondition: `caller` defined in `self.mod_env`
    fn visit_callees(
        &self,
        caller_node: NodeId,
        caller: QualifiedInstId<FunId>,
        insts: &[Type],
        callers_chain: &mut Vec<(Loc, QualifiedInstId<FunId>)>,
    ) -> bool {
        let fun_env = self.mod_env.get_function(caller.id);
        if let Some(caller_body) = fun_env.get_def() {
            let caller_loc = self.mod_env.env.get_node_loc(caller_node);
            callers_chain.push((caller_loc, caller));
            let insts = Type::instantiate_vec(self.get_inst(caller_node), insts);
            let res = caller_body.visit_positions_all_visits_return_true(&mut |pos, exp| {
                self.visit(pos, exp, &insts, callers_chain)
            });
            callers_chain.pop();
            res
        } else {
            true
        }
    }

    /// Shortcut for getting the node instantiation
    fn get_inst(&self, nid: NodeId) -> Vec<Type> {
        self.mod_env.env.get_node_instantiation(nid)
    }

    /// Returns true if we are sure the function with given id is not recursive
    fn def_not_recursive(&self, id: QualifiedId<FunId>) -> bool {
        !self
            .mod_env
            .env
            .get_function(id)
            .get_transitive_closure_of_called_functions()
            .contains(&id)
    }

    /// Reports a cyclic type instantiation error, in which the root caller eventually calls `callee`
    /// with a cyclic type instantiation. `callee` is the callee of the last caller in `callers_chain`.
    /// Precondition: `callers_chain` is not empty
    fn report_error(
        &self,
        nid: NodeId,
        callee: QualifiedInstId<FunId>,
        callers_chain: &mut [(Loc, QualifiedInstId<FunId>)],
    ) {
        let root = callers_chain[0].1.id;
        let mut labels = (0..callers_chain.len() - 1)
            .map(|i| {
                let (_caller_loc, caller) = &callers_chain[i];
                // callee of `caller`
                let (callee_loc, callee) = &callers_chain[i + 1];
                format!(
                    "`{}` calls `{}` {}",
                    self.display_call(caller, root),
                    self.display_call(callee, root),
                    callee_loc.display_file_name_and_line(self.mod_env.env)
                )
            })
            .collect_vec();
        let (_caller_loc, caller) = &callers_chain.last().expect("parent");
        let callee_loc = self.mod_env.env.get_node_loc(nid);
        labels.push(format!(
            "`{}` calls `{}` {}",
            self.display_call(caller, root),
            self.display_call(&callee, root),
            callee_loc.display_file_name_and_line(self.mod_env.env)
        ));
        let root_loc = self
            .mod_env
            .get_function(callers_chain[0].1.id)
            .get_id_loc();
        self.mod_env
            .env
            .error_with_notes(
                &root_loc,
                "cyclic type instantiation: a cycle of recursive calls causes a type to grow without bound",
                labels
            )
    }

    /// Returns the display name of a function call with type parameters but without arguments
    fn display_call(&self, call: &QualifiedInstId<FunId>, root_call: FunId) -> String {
        let fun_env = self.mod_env.get_function(call.id);
        let fun_name = fun_env.get_name_str();
        let root_env = self.mod_env.get_function(root_call);
        let type_disply_ctx = root_env.get_type_display_ctx();
        format!(
            "{}<{}>",
            fun_name,
            call.inst
                .iter()
                .map(|ty| ty.display(&type_disply_ctx).to_string())
                .join(", ")
        )
    }
}

/// Checks if the given type contains type parameters, and returns one if it does.
fn ty_contains_ty_parameter(ty: &Type) -> Option<u16> {
    match ty {
        Type::TypeParameter(i) => Some(*i),
        Type::Vector(ty) => ty_contains_ty_parameter(ty),
        Type::Struct(_, _, insts) => insts.iter().filter_map(ty_contains_ty_parameter).next(),
        Type::Primitive(_) => None,
        _ => panic!("ICE: {:?} used as a type parameter", ty),
    }
}

/// Checks if the given type properly contains type parameters, and returns one if it does.
fn ty_properly_contains_ty_parameter(ty: &Type) -> Option<u16> {
    match ty {
        Type::Vector(ty) => ty_contains_ty_parameter(ty),
        Type::Struct(_, _, insts) => insts.iter().filter_map(ty_contains_ty_parameter).next(),
        Type::Primitive(_) | Type::TypeParameter(_) => None,
        _ => panic!("ICE: {:?} used as a type parameter", ty),
    }
}
