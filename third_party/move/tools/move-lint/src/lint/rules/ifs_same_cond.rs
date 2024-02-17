//! Detect consecutive 'if' statements with identical conditions are usually redundant and can be
//! refactored to improve code readability and maintainability.
use crate::lint::utils::{add_diagnostic_and_emit, get_var_info_from_func_param, LintConfig};
use crate::lint::visitor::ExpressionAnalysisVisitor;
use move_model::ast::ExpData;
use move_model::model::{FunctionEnv, GlobalEnv};

pub struct IfsSameCondVisitor {
    if_condition: Vec<String>,
}

impl Default for IfsSameCondVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl IfsSameCondVisitor {
    pub fn new() -> Self {
        Self {
            if_condition: Vec::new(),
        }
    }
    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    /// Checks if the current 'if' condition is a duplicate and sets the condition for future checks.
    fn check_and_set_condition(&mut self, exp: &ExpData, func_env: &FunctionEnv, env: &GlobalEnv) {
        let current_condition = self.get_condition_string(exp, env, func_env);
        let founded_item = self.if_condition.contains(&current_condition);
        if founded_item {
            let message =
                "Detected consecutive if conditions with the same expression. Consider refactoring to avoid redundancy.";
            add_diagnostic_and_emit(
                &env.get_node_loc(exp.node_id()),
                message,
                codespan_reporting::diagnostic::Severity::Warning,
                env,
            );
        } else {
            self.if_condition.push(current_condition);
        }
    }

    /// Constructs a string representation of the given condition for comparison purposes.
    fn get_condition_string(
        &mut self,
        exp: &ExpData,
        env: &GlobalEnv,
        func_env: &FunctionEnv,
    ) -> String {
        match exp {
            ExpData::Call(_, oper, vec_exp) => {
                let mut vars = vec_exp
                    .iter()
                    .map(|e| match e.as_ref() {
                        ExpData::LocalVar(_, symbol) => {
                            env.symbol_pool().string(*symbol).to_string()
                        },
                        ExpData::Temporary(_, usize) => {
                            let parameters = func_env.get_parameters();
                            let param = get_var_info_from_func_param(*usize, &parameters);
                            if let Some(param) = param {
                                env.symbol_pool().string(param.0).to_string()
                            } else {
                                String::new()
                            }
                        },
                        ExpData::Value(_, value) => env.display(value).to_string(),
                        _ => String::new(),
                    })
                    .collect::<Vec<_>>();
                vars.sort();
                let exp_string = format!("{:?} {:?}", vars, oper);
                exp_string
            },
            _ => String::new(),
        }
    }

    fn clear_if_condition(&mut self) {
        self.if_condition.clear();
    }
}

impl ExpressionAnalysisVisitor for IfsSameCondVisitor {
    fn visit_function_custom(&mut self, func_env: &FunctionEnv, env: &GlobalEnv, _: &LintConfig) {
        let func = func_env.get_def();
        if let Some(func) = func {
            func.visit_pre_post(
                &mut (|up: bool, exp: &ExpData| {
                    if !up {
                        if let ExpData::IfElse(_, cond, _, _) = exp {
                            self.check_and_set_condition(cond.as_ref(), func_env, env);
                        }
                    }
                }),
            );
            self.clear_if_condition()
        }
    }
}
