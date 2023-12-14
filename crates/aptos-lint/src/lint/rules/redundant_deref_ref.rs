use crate::lint::visitor::{ExpDataVisitor, LintUtilities};
/// Detect unnecessary *&x patterns where x can be used directly instead.
use move_model::{
    ast::{ExpData, Operation},
    model::{FunctionEnv, GlobalEnv},
};
#[derive(Debug)]
pub struct RedundantDerefRefVisitor;

impl Default for RedundantDerefRefVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl RedundantDerefRefVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpDataVisitor> {
        Box::new(Self::new())
    }

    fn check_redundant_deref_ref(&self, exp: &ExpData, env: &GlobalEnv) {
        if let ExpData::Call(_, Operation::Deref, exp_vec) = exp {
            if let Some(ExpData::Call(_, Operation::Borrow(_), _)) =
                exp_vec.get(0).map(|e| e.as_ref())
            {
                let message =
                    "Redundant dereference of a reference detected (`*&` or `*&mut`). Consider simplifying the expression.";
                self.add_diagnostic_and_emit(
                    &env.get_node_loc(exp.node_id()),
                    message,
                    codespan_reporting::diagnostic::Severity::Warning,
                    env,
                );
            }
        }
    }
}

impl ExpDataVisitor for RedundantDerefRefVisitor {
    fn visit(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        if let Some(func) = func_env.get_def().as_ref() {
            func.visit_pre_post(
                &mut (|up: bool, exp: &ExpData| {
                    if up {
                        self.check_redundant_deref_ref(exp, env);
                    }
                }),
            );
        }
    }
}

impl LintUtilities for RedundantDerefRefVisitor {}
