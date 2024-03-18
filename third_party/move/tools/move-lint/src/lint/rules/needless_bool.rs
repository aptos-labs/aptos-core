//! Defines `NeedlessBoolVisitor` for linting Move code, specifically targeting unnecessary boolean comparisons.
//! Identifies `if` expressions comparing variables or function returns directly with `true` or `false` and suggests simplifications.
//! Supports simplifying `if x { true } else { false }` to `x` and `if x { false } else { true }` to `!x`.
use crate::lint::{
    utils::{add_diagnostic_and_emit, get_var_name_or_func_name_from_exp, LintConfig},
    visitor::ExpressionAnalysisVisitor,
};
use codespan::FileId;
use codespan_reporting::diagnostic::Diagnostic;
use move_model::{
    ast::{ExpData, Value},
    model::{FunctionEnv, GlobalEnv},
};
pub struct NeedlessBoolVisitor;

impl Default for NeedlessBoolVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl NeedlessBoolVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn check_conditional_simplification(
        &mut self,
        exp: &ExpData,
        func_env: &FunctionEnv,
        env: &GlobalEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        if let ExpData::IfElse(_, cond, if_exp, else_exp) = exp {
            // Check if the branches are simple true/false literals
            let var_name = get_var_name_or_func_name_from_exp(cond, func_env, env)
                .expect("Expected to get a variable name");
            match (if_exp.as_ref(), else_exp.as_ref()) {
                (ExpData::Value(_, Value::Bool(true)), ExpData::Value(_, Value::Bool(false))) => {
                    let diagnostic_msg = format!(
                        "Simplify `if {} {{ true }} else {{ false }}` to `{}`.",
                        var_name, var_name
                    );
                    add_diagnostic_and_emit(
                        &env.get_node_loc(cond.node_id()),
                        &diagnostic_msg,
                        codespan_reporting::diagnostic::Severity::Warning,
                        env,
                        diags,
                    );
                },
                (ExpData::Value(_, Value::Bool(false)), ExpData::Value(_, Value::Bool(true))) => {
                    let diagnostic_msg = format!(
                        "Simplify `if {} {{ false }} else {{ true }}` to `!{}`.",
                        var_name, var_name
                    );
                    add_diagnostic_and_emit(
                        &env.get_node_loc(cond.node_id()),
                        &diagnostic_msg,
                        codespan_reporting::diagnostic::Severity::Warning,
                        env,
                        diags,
                    );
                },
                _ => {}, // No action for other patterns
            }
        }
    }
}

impl ExpressionAnalysisVisitor for NeedlessBoolVisitor {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        func_env: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        self.check_conditional_simplification(exp, func_env, env, diags);
    }
}
