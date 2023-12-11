// Detects comparisons where a variable is compared to 'true' or 'false' using
// equality (==) or inequality (!=) operators and provides suggestions to simplify the comparisons.
use move_model::ast::{ ExpData, Operation, Value };
use move_model::model::{ GlobalEnv, FunctionEnv };
use crate::lint::visitor::{ LintUtilities, ExpDataVisitor };

pub struct BoolComparisonVisitor;
impl BoolComparisonVisitor {
    pub fn new() -> Self {
        Self {}
    }
    pub fn visitor() -> Box<dyn ExpDataVisitor> {
        Box::new(Self::new())
    }

    fn check_boolean_comparison(&mut self, exp: &ExpData, func_env: &FunctionEnv, env: &GlobalEnv) {
        match &exp {
            ExpData::Call(_, oper, vec_exp) => {
                if let Some(exp_val) = vec_exp.get(1) {
                    if let ExpData::Value(_, val) = &exp_val.as_ref() {
                        if let Value::Bool(b) = &val {
                            let var_name = match vec_exp[0].as_ref() {
                                ExpData::Temporary(_, index) => {
                                    let param = self
                                        .get_var_info_from_func_param(index, func_env.get_parameters())
                                        .unwrap();
                                    env.symbol_pool().string(param.0).to_string()
                                }
                                ExpData::LocalVar(_, sym) => env.symbol_pool().string(*sym).to_string(),
                                _ => String::from(""),
                            };

                            let diagnostic_msg = match (oper, b) {
                                (Operation::Eq, true) | (Operation::Neq, false) => {
                                    format!("Use {} directly instead of comparing it to {}.", var_name, b)
                                }
                                (Operation::Eq, false) | (Operation::Neq, true) => {
                                    format!("Use !{} instead of comparing it to {}.", var_name, b)
                                }
                                _ => String::from(""),
                            };

                            if !diagnostic_msg.is_empty() {
                                self.add_diagnostic_and_emit(
                                    &env.get_node_loc(exp.node_id()),
                                    &diagnostic_msg,
                                    codespan_reporting::diagnostic::Severity::Warning,
                                    env
                                );
                            }
                        }
                    }
                }
            }
            _ => (),
        }
    }
}

impl ExpDataVisitor for BoolComparisonVisitor {
    fn visit(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        let mut visitor = |t: &ExpData| {
            match t {
                ExpData::IfElse(_, e1, _, _) => {
                    self.check_boolean_comparison(e1.as_ref(), func_env, env);
                }
                _ => (),
            }
        };
        func_env.get_def().map(|func| {
            func.visit(&mut visitor);
        });
    }
}

impl LintUtilities for BoolComparisonVisitor {}
