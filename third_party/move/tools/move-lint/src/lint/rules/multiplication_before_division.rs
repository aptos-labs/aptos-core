//! Detect expressions where multiplication appears before division, which can magnify rounding error.
use crate::lint::utils::{add_diagnostic_and_emit, LintConfig};
use crate::lint::visitor::ExpressionAnalysisVisitor;
use codespan::FileId;

use codespan_reporting::diagnostic::Diagnostic;
use move_model::ast::{Exp, ExpData, Operation};
use move_model::model::{FunctionEnv, GlobalEnv};
pub struct MultiplicationBeforeDivisionVisitor;

impl Default for MultiplicationBeforeDivisionVisitor {
    fn default() -> Self {
        Self::new()
    }
}

fn has_binop_div_in_exp(exp: &Exp) -> bool {
    match exp.as_ref() {
        ExpData::Call(_, op, e2) => match op {
            Operation::Div => true,
            _ => has_binop_div_in_exp(&e2[0]),
        },
        _ => false,
    }
}

impl MultiplicationBeforeDivisionVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn check_multiplication_before_division(
        &mut self,
        exp: &ExpData,
        env: &GlobalEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        if let ExpData::Block(_, _, Some(call), _) = &exp {
            if let ExpData::Call(_, Operation::Mul, exp_vec) = call.as_ref() {
                if has_binop_div_in_exp(&exp_vec[0]) {
                    let message =
                        "Multiplication should come before division to avoid rounding errors.";
                    add_diagnostic_and_emit(
                        &env.get_node_loc(call.node_id()),
                        message,
                        codespan_reporting::diagnostic::Severity::Warning,
                        env,
                        diags,
                    );
                }
            }
        }
    }
}

impl ExpressionAnalysisVisitor for MultiplicationBeforeDivisionVisitor {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        _func_env: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        self.check_multiplication_before_division(exp, env, diags);
    }
}
