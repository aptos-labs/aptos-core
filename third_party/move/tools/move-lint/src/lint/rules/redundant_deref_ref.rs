//! Detect unnecessary *&x patterns where x can be used directly instead.
use crate::lint::{
    utils::{add_diagnostic_and_emit, LintConfig},
    visitor::ExpressionAnalysisVisitor,
};
use codespan::FileId;

use codespan_reporting::diagnostic::Diagnostic;
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

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn check_redundant_deref_ref(
        &self,
        exp: &ExpData,
        env: &GlobalEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        if let ExpData::Call(_, Operation::Deref, exp_vec) = exp {
            if let Some(ExpData::Call(_, Operation::Borrow(_), _)) =
                exp_vec.get(0).map(|e| e.as_ref())
            {
                let message =
                    "Redundant dereference of a reference detected (`*&` or `*&mut`). Consider simplifying the expression.";
                add_diagnostic_and_emit(
                    &env.get_node_loc(exp.node_id()),
                    message,
                    codespan_reporting::diagnostic::Severity::Warning,
                    env,
                    diags,
                );
            }
        }
    }
}

impl ExpressionAnalysisVisitor for RedundantDerefRefVisitor {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        _: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        self.check_redundant_deref_ref(exp, env, diags);
    }
}
