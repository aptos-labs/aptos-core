use crate::lint::utils::{add_diagnostic_and_emit, LintConfig};
use crate::lint::visitor::ExpressionAnalysisVisitor;
use move_model::ast::{ExpData, Value};
use move_model::model::{FunctionEnv, GlobalEnv};

pub struct UnnecessaryWhileTrueVisitor;

impl Default for UnnecessaryWhileTrueVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl UnnecessaryWhileTrueVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    /// Checks for `while(true)` loops.
    fn check_unnecessary_while_true(&self, exp: &ExpData, env: &GlobalEnv) {
        if let ExpData::Loop(_, body) = exp {
            if let ExpData::IfElse(_, cond, _, _) = body.as_ref() {
                if let ExpData::Value(_, Value::Bool(true)) = cond.as_ref() {
                    let message =
                        "Unnecessary 'while(true)' detected. Consider using 'loop' instead.";
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

impl ExpressionAnalysisVisitor for UnnecessaryWhileTrueVisitor {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        _func_env: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
    ) {
        self.check_unnecessary_while_true(exp, env);
    }
}
