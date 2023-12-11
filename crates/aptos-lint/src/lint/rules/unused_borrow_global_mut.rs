use move_model::ast::{ ExpData, Pattern, Operation };
use move_model::model::{ GlobalEnv, Loc, NodeId, FunctionEnv };
use move_model::ty::{ ReferenceKind, Type };
use crate::lint::visitor::{ ExpDataVisitor, LintUtilities };

#[derive(Debug)]
pub struct UnusedBorrowGlobalMutVisitor {
    variables: Vec<VarInfo>,
}

#[derive(Debug, PartialEq, Clone, Eq, Hash)]
struct VarInfo {
    node_id: NodeId,
    loc: Loc,
    var_name: String,
    used: bool,
}
impl VarInfo {
    fn mark_used(&mut self) {
        self.used = true;
    }
}

impl UnusedBorrowGlobalMutVisitor {
    pub fn new() -> Self {
        Self {
            variables: Vec::new(),
        }
    }

    fn add_variable(&mut self, info: VarInfo) {
        self.variables.push(info);
    }

    fn find_nearest_declaration(&mut self, name: String) {
        for var in self.variables.iter_mut().rev() {
            if var.var_name == name {
                var.mark_used();
                break;
            }
        }
    }
    pub fn visitor() -> Box<dyn ExpDataVisitor> {
        Box::new(Self::new())
    }

    fn emit_warning(&mut self, env: &GlobalEnv) {
        self.variables
            .iter()
            .filter(|x| !x.used)
            .for_each(|var| {
                let node_type = env.get_node_type(var.node_id);
                match node_type {
                    Type::Reference(kind, _) => {
                        if ReferenceKind::Mutable == kind {
                            let message = format!(
                                "Unused borrowed mutable variable {}. Consider normal borrow (borrow_global, vector::borrow, etc.) instead",
                                var.var_name
                            );
                            self.add_diagnostic_and_emit(
                                &var.loc,
                                &message,
                                codespan_reporting::diagnostic::Severity::Warning,
                                env
                            );
                        }
                    }
                    _ => {}
                }
            });
    }

    fn clear_variables(&mut self) {
        self.variables.clear();
    }

    fn visit_exp_custom(&mut self, exp: &ExpData, env: &GlobalEnv) {
        match exp {
            ExpData::Call(_, oper, vec_exp) => {
                match oper {
                    Operation::MoveFunction(_, _) => {
                        eprintln!(" vec_exp: {:?}", vec_exp);
                        vec_exp.iter().for_each(|exp| {
                            match exp.as_ref() {
                                ExpData::LocalVar(_, sym) => {
                                    let var_name = env.symbol_pool().string(*sym).to_string();
                                    self.find_nearest_declaration(var_name)
                                }
                                _ => {}
                            }
                        });
                    }
                    _ => {}
                }
            }
            ExpData::Block(_, pattern, _, _) => {
                match pattern {
                    Pattern::Var(node_id, sym) => {
                        let var_name = env.symbol_pool().string(*sym).to_string();
                        self.add_variable(VarInfo {
                            node_id: *node_id,
                            loc: env.get_node_loc(*node_id),
                            var_name: var_name.clone(),
                            used: false,
                        });
                    }
                    _ => {}
                }
            }
            ExpData::Assign(_, pattern, _) => {
                match pattern {
                    Pattern::Var(_, sym) => {
                        let var_name = env.symbol_pool().string(*sym).to_string();
                        self.find_nearest_declaration(var_name)
                    }
                    _ => {}
                }
            }
            ExpData::Mutate(_, exp, _) => {
                match exp.as_ref() {
                    ExpData::LocalVar(_, sym) => {
                        let var_name = env.symbol_pool().string(*sym).to_string();
                        self.find_nearest_declaration(var_name)
                    }
                    _ => {}
                }
            }

            _ => {}
        }
    }
}

impl ExpDataVisitor for UnusedBorrowGlobalMutVisitor {
    fn visit(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        func_env.get_def().map(|func| {
            func.visit_pre_post(
                &mut (|up: bool, exp: &ExpData| {
                    if !up {
                        self.visit_exp_custom(exp, env);
                    }
                })
            );
            self.emit_warning(env);
            self.clear_variables();
        });
    }
}

impl LintUtilities for UnusedBorrowGlobalMutVisitor {}
