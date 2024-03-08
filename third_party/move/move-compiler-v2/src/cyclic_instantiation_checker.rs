use itertools::Itertools;
use move_model::{
    ast::{Exp, ExpData, Operation, VisitorPosition},
    model::{FunId, GlobalEnv, Loc, ModuleEnv, NodeId, QualifiedId, QualifiedInstId},
    ty::Type,
};

/// Checks all modules in `env`
pub fn check_cyclic_instantiations(env: &GlobalEnv) {
	for module in env.get_modules() {
		let checker = CyclicInstantiationChecker::new(module);
		checker.check();
	}
}

/// Module checker state
struct CyclicInstantiationChecker<'a> {
	/// The module we are checking
    mod_env: ModuleEnv<'a>,
}

impl<'a> CyclicInstantiationChecker<'a> {
	pub fn new(mod_env: ModuleEnv<'a>) -> Self {
		Self {
			mod_env,
		}
	}

	/// Checks all functions in the module
	fn check(&self) {
		for fun_env in self.mod_env.get_functions() {
			self.check_fun(fun_env.get_id())
		}
	}

	/// Checks the given function
    fn check_fun(&self, fun_id: FunId) {
        let fun_body = self.get_fun_def(fun_id);
        let num_ty_params = self.mod_env.get_function(fun_id).get_type_parameter_count() as u16;
        let insts = (0..num_ty_params).map(Type::TypeParameter).collect_vec();
		let root_caller = self.mod_env.get_id().qualified_inst(fun_id, insts.clone());
		let root_caller_loc = self.mod_env.get_function(fun_id).get_loc();
        let mut callers = vec![(root_caller_loc, root_caller)];
        fun_body.visit_positions(&mut |pos, e| self.visit(pos, e, &mut callers, insts.clone()));
    }

	/// Visits an expression and checks for cyclic type instantiations.
	/// `callers_chain`: the chain of callers leading to the current expression
	/// `insts`: the type parameters of the current expression
    fn visit(
        &self,
        position: VisitorPosition,
        e: &ExpData,
        callers_chain: &mut Vec<(Loc, QualifiedInstId<FunId>)>,
        insts: Vec<Type>,
    ) -> bool {
        use ExpData::*;
        use VisitorPosition::*;
        match (position, e) {
            (Pre, Call(nid, op, _)) => self.visit_call(nid, op, callers_chain, insts),
            _ => true,
        }
    }

    /// Visits a call expression and checks for cyclic type instantiations.
	/// Other parameters are the same as in `visit`
    fn visit_call(
        &self,
        nid: &NodeId,
        op: &Operation,
        callers_chain: &mut Vec<(Loc, QualifiedInstId<FunId>)>,
        insts: Vec<Type>,
    ) -> bool {
        if let Operation::MoveFunction(mod_id, fun_id) = op {
            let callee_uninst = mod_id.qualified_inst(*fun_id, self.get_inst(*nid));
            let callee = callee_uninst.instantiate(&insts);
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
                self.visit_callees(*nid, callee, callers_chain, insts)
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
        callers_chain: &mut Vec<(Loc, QualifiedInstId<FunId>)>,
        insts: Vec<Type>,
    ) -> bool {
        let caller_body = self.get_fun_def(caller.id);
		let caller_loc = self.mod_env.env.get_node_loc(caller_node);
        callers_chain.push((caller_loc, caller));
        let insts = Type::instantiate_vec(self.get_inst(caller_node), &insts);
        let res = caller_body
            .visit_positions(&mut |pos, exp| self.visit(pos, exp, callers_chain, insts.clone()));
        callers_chain.pop();
        res.is_some()
    }

    /// Gets the funciton body
    fn get_fun_def(&self, fun_id: FunId) -> &Exp {
        let fun_env = self.mod_env.get_function(fun_id);
        fun_env.data.get_def().expect("definition")
    }

    /// Shortcut for getting the node instantiation
    fn get_inst(&self, nid: NodeId) -> Vec<Type> {
        self.mod_env
            .env
            .get_node_instantiation_opt(nid)
            .expect("instantiation")
    }

    /// Returns true if we are sure the function with given id is not recursive
    fn def_not_recursive(&self, id: QualifiedId<FunId>) -> bool {
        if let Some(descendants) = self
            .mod_env
            .env
            .get_function(id)
            .get_transitive_closure_of_called_functions()
        {
            !descendants.contains(&id)
        } else {
            false
        }
    }

	/// Reports a cyclic type instantiation error, in which the root caller eventually calls `callee`
	/// with a cyclic type instantiation.
    fn report_error(
        &self,
        _nid: NodeId,
        callee: QualifiedInstId<FunId>,
        callers_chain: &mut Vec<(Loc, QualifiedInstId<FunId>)>,
    ) {
        let labels = (0..callers_chain.len())
            .map(|i| {
                let (caller_loc, caller) = &callers_chain[i];
                let callee = if i != callers_chain.len() - 1 {
                    &callers_chain[i + 1].1
                } else {
                    &callee
                };
                format!(
                    "{} calls {} {}",
                    self.display_call(&caller),
                    self.display_call(&callee),
                    caller_loc.display_line_only(&self.mod_env.env)
                )
            })
            .collect_vec();
        let root_loc = &callers_chain[0].0;
        self.mod_env
            .env
            .error_with_notes(&root_loc, &format!("cyclic type instantiation {}", self.mod_env.get_function(callers_chain[0].1.id).get_name_str()), labels)
    }

    /// Returns the display name of a function call with type parameters but without arguments
    fn display_call(&self, call: &QualifiedInstId<FunId>) -> String {
        let fun_env = self.mod_env.get_function(call.id);
        let fun_name = fun_env.get_name_str();
        let type_disply_ctx = fun_env.get_type_display_ctx();
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
