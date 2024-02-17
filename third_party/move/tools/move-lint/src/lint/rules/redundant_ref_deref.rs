//! Detect unnecessary expressions &*x where x is a reference or mutable reference.
//! This can be simplified to using x directly, regardless of whether x has copy ability.
use crate::lint::{
    utils::{add_diagnostic_and_emit, LintConfig},
    visitor::ExpressionAnalysisVisitor,
};
use move_model::{
    ast::{ExpData, Operation},
    model::{FunctionEnv, GlobalEnv},
};
#[derive(Debug)]
pub struct RedundantRefDerefVisitor;

impl Default for RedundantRefDerefVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl RedundantRefDerefVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn check_borrow_deref_ref(&self, exp: &ExpData, env: &GlobalEnv) {
        if let ExpData::Call(_, Operation::Borrow(_), exp_vec) = exp {
            if let Some(ExpData::Call(_, Operation::Deref, inner_exp_vec)) =
                exp_vec.get(0).map(|e| e.as_ref())
            {
                if let Some(ExpData::Call(_, Operation::Borrow(_), _)) =
                    inner_exp_vec.get(0).map(|e| e.as_ref())
                {
                    let message = "Redundant borrow-dereference detected. Consider removing the borrow-dereference operation and using the expression directly.";
                    add_diagnostic_and_emit(
                        &env.get_node_loc(exp.node_id()),
                        message,
                        codespan_reporting::diagnostic::Severity::Warning,
                        env,
                    );
                }
            }
        }
    }
}

impl ExpressionAnalysisVisitor for RedundantRefDerefVisitor {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        _: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
    ) {
        self.check_borrow_deref_ref(exp, env);
    }
}
