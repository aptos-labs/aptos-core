use move_model::{ model::{ FunctionEnv, GlobalEnv }, ast::ExpData };
use regex::Regex;

use crate::lint::visitor::{ ExpDataVisitor, LintUtilities };

#[derive(Debug)]
pub struct DoubleParenthesesVisitor;

impl DoubleParenthesesVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpDataVisitor> {
        Box::new(Self::new())
    }

    fn check_double_parentheses(&self, exp: &ExpData, env: &GlobalEnv) {
        let exp_str = env.get_source(&env.get_node_loc(exp.node_id())).unwrap();
        match exp {
            ExpData::Call(node_id, _, _) => {
                let re = Regex::new(r"\(\s*\(\s*([^)]+?)\s*\)\s*\)").unwrap();
                if re.is_match(exp_str) {
                    let message = "Unnecessary double parentheses detected. Consider simplifying the expression.";
                    self.add_diagnostic_and_emit(
                        &env.get_node_loc(*node_id),
                        message,
                        codespan_reporting::diagnostic::Severity::Warning,
                        env
                    );
                }
            }
            _ => {}
        }
    }

}

impl ExpDataVisitor for DoubleParenthesesVisitor {
    fn visit(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        func_env.get_def().map(|func| {
            func.visit_pre_post(
                &mut (|up: bool, exp: &ExpData| {
                    if !up {
                        self.check_double_parentheses(exp, env);
                    }
                })
            );
        });
    }
}

impl LintUtilities for DoubleParenthesesVisitor {}
