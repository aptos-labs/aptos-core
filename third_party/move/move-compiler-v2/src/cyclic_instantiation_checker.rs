use itertools::Itertools;
use move_model::{
    ast::{Exp, ExpData, Operation, VisitorPosition},
    model::{FunId, GlobalEnv, Loc, ModuleEnv, NodeId, QualifiedId, QualifiedInstId},
    ty::Type,
};


pub fn check_cyclic_instantiations(env: &GlobalEnv) {
	for module in env.get_modules() {
		let checker = CyclicInstantiationChecker::new(module);
		checker.check();
	}
}

struct CyclicInstantiationChecker<'a> {
    mod_env: ModuleEnv<'a>,
}

impl<'a> CyclicInstantiationChecker<'a> {
	pub fn new(mod_env: ModuleEnv<'a>) -> Self {
		Self {
			mod_env,
		}
	}

	fn check(&self) {
		for fun_env in self.mod_env.get_functions() {
			self.check_fun(fun_env.get_id())
		}
	}

    fn check_fun(&self, fun_id: FunId) {
        let fun_body = self.get_fun_def(fun_id);
        let mut callers = Vec::new();
        let num_ty_params = self.mod_env.get_function(fun_id).get_type_parameter_count() as u16;
        let insts = (0..num_ty_params).map(Type::TypeParameter).collect_vec();
		println!("38");
        fun_body.visit_positions(&mut |pos, e| self.visit(pos, e, &mut callers, insts.clone()));
    }

    fn visit(
        &self,
        position: VisitorPosition,
        e: &ExpData,
        callers_chain: &mut Vec<(NodeId, QualifiedInstId<FunId>)>,
        insts: Vec<Type>,
    ) -> bool {
        use ExpData::*;
        use VisitorPosition::*;
		println!("visit {:?} with {:?}", e, insts);
        match (position, e) {
            (Pre, Call(nid, op, _)) => self.visit_call(nid, op, callers_chain, insts),
            _ => true,
        }
    }

    /// `insts`: type parameters of the call
    fn visit_call(
        &self,
        nid: &NodeId,
        op: &Operation,
        callers_chain: &mut Vec<(NodeId, QualifiedInstId<FunId>)>,
        insts: Vec<Type>,
    ) -> bool {
        if let Operation::MoveFunction(mod_id, fun_id) = op {
            let callee_uninst = mod_id.qualified_inst(*fun_id, self.get_inst(*nid));
            let callee = callee_uninst.instantiate(&insts);
            if *mod_id != self.mod_env.get_id() || self.def_not_recursive(callee.to_qualified_id())
            {
                // skips if callee from another module (since there is no cyclic module dependency),
                // or callee
                true
            } else {
                for (_, ancester_caller) in callers_chain.iter() {
                    if ancester_caller.to_qualified_id() == callee.to_qualified_id() {
                        // we are checking for the root caller
                        let (_, checking_for) = &callers_chain[0];
                        if checking_for.to_qualified_id() != callee.to_qualified_id() {
                            // check and report diagnostics when `callee` is checked
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

    /// Checks calles of `caller`
    /// Precondition: `caller` defined in `self.moc_env`
    fn visit_callees(
        &self,
        caller_node: NodeId,
        caller: QualifiedInstId<FunId>,
        callers_chain: &mut Vec<(NodeId, QualifiedInstId<FunId>)>,
        insts: Vec<Type>,
    ) -> bool {
        let caller_body = self.get_fun_def(caller.id);
        callers_chain.push((caller_node, caller));
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

    fn report_error(
        &self,
        nid: NodeId,
        callee: QualifiedInstId<FunId>,
        callers_chain: &mut Vec<(NodeId, QualifiedInstId<FunId>)>,
    ) {
        let labels = (0..callers_chain.len())
            .map(|i| {
                let (caller_node, caller) = &callers_chain[i];
                let caller_loc = self.mod_env.env.get_node_loc(*caller_node);
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
        let root_node_id = &callers_chain[0].0;
        let root_loc = self.mod_env.env.get_node_loc(*root_node_id);
        self.mod_env
            .env
            .error_with_notes(&root_loc, "cyclic type instantiation", labels)
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
