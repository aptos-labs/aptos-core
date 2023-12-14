use crate::lint::visitor::{ExpDataVisitor, LintUtilities};
/// Detect consecutive 'if' statements with identical conditions are usually redundant and can be
/// refactored to improve code readability and maintainability.
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

    pub fn visitor() -> Box<dyn ExpDataVisitor> {
        Box::new(Self::new())
    }

    fn check_and_set_condition(&mut self, exp: &ExpData, func_env: &FunctionEnv, env: &GlobalEnv) {
        let current_condition = self.get_condition_string(exp, env, func_env);
        let founded_item = self.if_condition.contains(&current_condition);
        if founded_item {
            let message =
                "Detected consecutive if conditions with the same expression. Consider refactoring to avoid redundancy.";
            self.add_diagnostic_and_emit(
                &env.get_node_loc(exp.node_id()),
                message,
                codespan_reporting::diagnostic::Severity::Warning,
                env,
            );
        } else {
            self.if_condition.push(current_condition);
        }
    }

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
                            let param =
                                self.get_var_info_from_func_param(usize, func_env.get_parameters());
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

impl ExpDataVisitor for IfsSameCondVisitor {
    fn visit(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        let mut visitor = |t: &ExpData| {
            if let ExpData::IfElse(_, e1, _, _) = t {
                self.check_and_set_condition(e1.as_ref(), func_env, env);
                self.clear_if_condition()
            }
        };
        if let Some(func) = func_env.get_def().as_ref() {
            func.visit(&mut visitor);
        };
    }
}

impl LintUtilities for IfsSameCondVisitor {}
