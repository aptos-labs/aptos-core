//! This lint identifies and warns about redundant return statements at the end of functions in Move programs.
//! It aims to improve code clarity by suggesting the removal of unnecessary return expressions.
use crate::lint::utils::{add_diagnostic_and_emit, LintConfig};
use crate::lint::visitor::ExpressionAnalysisVisitor;
use move_model::ast::ExpData;
use move_model::model::{FunctionEnv, GlobalEnv};

pub struct ReturnAtEndOfBlockVisitor {
    exp_in_function: Vec<ExpData>,
}

impl Default for ReturnAtEndOfBlockVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ReturnAtEndOfBlockVisitor {
    pub fn new() -> Self {
        Self {
            exp_in_function: vec![],
        }
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn clear_exp_in_function(&mut self) {
        self.exp_in_function.clear();
    }

    /// Checks if the last expression in a function is a return expression.
    fn check_return_at_end_of_function(&mut self, env: &GlobalEnv) {
        if self.exp_in_function.is_empty() || self.exp_in_function.len() < 2 {
            return;
        }
        let return_exp = self.exp_in_function.get(self.exp_in_function.len() - 2);
        if let Some(ExpData::Return(_, _)) = return_exp {
            let message =
                "Return statement at the end of the function is redundant. Consider removing it.";
            add_diagnostic_and_emit(
                &env.get_node_loc(return_exp.expect("Return expression not found").node_id()),
                message,
                codespan_reporting::diagnostic::Severity::Warning,
                env,
            );
        }
        self.clear_exp_in_function();
    }
}

impl ExpressionAnalysisVisitor for ReturnAtEndOfBlockVisitor {
    fn visit_function_custom(&mut self, func_env: &FunctionEnv, env: &GlobalEnv, _: &LintConfig) {
        let func = func_env.get_def();
        if let Some(func) = func.as_ref() {
            func.visit_pre_post(
                &mut (|up: bool, exp: &ExpData| {
                    if !up {
                        self.exp_in_function.push(exp.clone());
                    }
                }),
            );
        }

        self.check_return_at_end_of_function(env);
    }
}
