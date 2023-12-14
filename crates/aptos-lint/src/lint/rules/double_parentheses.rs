use crate::lint::visitor::{ExpDataVisitor, LintUtilities};
/// Detect unnecessary double parentheses such as ((x + 1)).
use move_model::{
    ast::ExpData,
    model::{FunctionEnv, GlobalEnv},
};
use regex::Regex;

#[derive(Debug)]
pub struct DoubleParenthesesVisitor;

impl Default for DoubleParenthesesVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl DoubleParenthesesVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpDataVisitor> {
        Box::new(Self::new())
    }

    fn check_double_parentheses(&self, exp: &ExpData, env: &GlobalEnv) {
        let exp_str = env.get_source(&env.get_node_loc(exp.node_id())).unwrap();
        if let ExpData::Call(node_id, _, _) = exp {
            let re = Regex::new(r"\(\s*\(\s*([^)]+?)\s*\)\s*\)").unwrap();
            if re.is_match(exp_str) {
                let message = "Unnecessary double parentheses detected. Consider removing.";
                self.add_diagnostic_and_emit(
                    &env.get_node_loc(*node_id),
                    message,
                    codespan_reporting::diagnostic::Severity::Warning,
                    env,
                );
            }
        }
    }
}

impl ExpDataVisitor for DoubleParenthesesVisitor {
    fn visit(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        if let Some(func) = func_env.get_def().as_ref() {
            func.visit_pre_post(
                &mut (|up: bool, exp: &ExpData| {
                    if !up {
                        self.check_double_parentheses(exp, env);
                    }
                }),
            );
        };
    }
}

impl LintUtilities for DoubleParenthesesVisitor {}
