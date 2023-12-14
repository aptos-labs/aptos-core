use crate::lint::visitor::{ExpDataVisitor, LintUtilities};
/// Detect all unused mutable variables, including those from borrow_global_mut, table::borrow_mut,
/// vector::borrow_mut, etc.
/// In these cases, immutable references should be obtained and used instead.
use move_model::ast::{ExpData, Operation, Pattern};
use move_model::{
    model::{FunctionEnv, GlobalEnv, Loc, NodeId},
    ty::{ReferenceKind, Type},
};

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

impl Default for UnusedBorrowGlobalMutVisitor {
    fn default() -> Self {
        Self::new()
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
                if let Type::Reference(kind, _) = node_type {
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
            });
    }

    fn clear_variables(&mut self) {
        self.variables.clear();
    }

    fn visit_exp_custom(&mut self, exp: &ExpData, env: &GlobalEnv) {
        match exp {
            ExpData::Call(_, Operation::MoveFunction(_, _), vec_exp) => {
                vec_exp.iter().for_each(|exp| {
                    if let ExpData::LocalVar(_, sym) = exp.as_ref() {
                        let var_name = env.symbol_pool().string(*sym).to_string();
                        self.find_nearest_declaration(var_name)
                    }
                });
            },
            ExpData::Block(_, Pattern::Var(node_id, sym), _, _) => {
                let var_name = env.symbol_pool().string(*sym).to_string();
                self.add_variable(VarInfo {
                    node_id: *node_id,
                    loc: env.get_node_loc(*node_id),
                    var_name: var_name.clone(),
                    used: false,
                });
            },
            ExpData::Assign(_, Pattern::Var(_, sym), _) => {
                let var_name = env.symbol_pool().string(*sym).to_string();
                self.find_nearest_declaration(var_name)
            },
            ExpData::Mutate(_, exp, _) => {
                if let ExpData::LocalVar(_, sym) = exp.as_ref() {
                    let var_name = env.symbol_pool().string(*sym).to_string();
                    self.find_nearest_declaration(var_name)
                }
            },

            _ => {},
        }
    }
}

impl ExpDataVisitor for UnusedBorrowGlobalMutVisitor {
    fn visit(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        if let Some(func) = func_env.get_def().as_ref() {
            func.visit_pre_post(
                &mut (|up: bool, exp: &ExpData| {
                    if !up {
                        self.visit_exp_custom(exp, env);
                    }
                }),
            );
            self.emit_warning(env);
            self.clear_variables();
        };
    }
}

impl LintUtilities for UnusedBorrowGlobalMutVisitor {}
