use crate::lint::visitor::{ExpDataVisitor, LintUtilities};
/// Detects comparisons where a variable is compared to 'true' or 'false' using
/// equality (==) or inequality (!=) operators and provides suggestions to simplify the comparisons.
/// Examples: if (x == true) can be simplified to if (x), if (x == false) can be simplified to if (!x)
use move_model::ast::{ExpData, Operation, Value};
use move_model::model::{FunctionEnv, GlobalEnv};
pub struct BoolComparisonVisitor;

impl Default for BoolComparisonVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl BoolComparisonVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpDataVisitor> {
        Box::new(Self::new())
    }

    fn check_boolean_comparison(&mut self, exp: &ExpData, func_env: &FunctionEnv, env: &GlobalEnv) {
        if let ExpData::Call(_, oper, vec_exp) = &exp {
            if let Some(exp_val) = vec_exp.get(1) {
                if let ExpData::Value(_, Value::Bool(b)) = &exp_val.as_ref() {
                    let var_name = match vec_exp[0].as_ref() {
                        ExpData::Temporary(_, index) => {
                            let param = self
                                .get_var_info_from_func_param(index, func_env.get_parameters())
                                .unwrap();
                            env.symbol_pool().string(param.0).to_string()
                        },
                        ExpData::LocalVar(_, sym) => env.symbol_pool().string(*sym).to_string(),
                        _ => String::from(""),
                    };

                    let diagnostic_msg = match (oper, b) {
                        (Operation::Eq, true) | (Operation::Neq, false) => {
                            format!(
                                "Use {} directly instead of comparing it to {}.",
                                var_name, b
                            )
                        },
                        (Operation::Eq, false) | (Operation::Neq, true) => {
                            format!("Use !{} instead of comparing it to {}.", var_name, b)
                        },
                        _ => String::from(""),
                    };

                    if !diagnostic_msg.is_empty() {
                        self.add_diagnostic_and_emit(
                            &env.get_node_loc(exp.node_id()),
                            &diagnostic_msg,
                            codespan_reporting::diagnostic::Severity::Warning,
                            env,
                        );
                    }
                } else {
                    for exp in vec_exp {
                        self.check_boolean_comparison(exp, func_env, env);
                    }
                }
            }
        }
    }
}

impl ExpDataVisitor for BoolComparisonVisitor {
    fn visit(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        let mut visitor = |t: &ExpData| {
            if let ExpData::IfElse(_, e1, _, _) = t {
                self.check_boolean_comparison(e1.as_ref(), func_env, env);
            }
        };
        if let Some(func) = func_env.get_def().as_ref() {
            func.visit(&mut visitor);
        };
    }
}

impl LintUtilities for BoolComparisonVisitor {}
