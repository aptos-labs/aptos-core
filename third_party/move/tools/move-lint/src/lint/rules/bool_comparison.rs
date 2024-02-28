//! Detects comparisons where a variable is compared to 'true' or 'false' using
//! equality (==) or inequality (!=) operators and provides suggestions to simplify the comparisons.
//! Examples: if (x == true) can be simplified to if (x), if (x == false) can be simplified to if (!x)
use crate::lint::utils::{add_diagnostic_and_emit, get_var_info_from_func_param, LintConfig};
use crate::lint::visitor::ExpressionAnalysisVisitor;

use codespan::FileId;
use codespan_reporting::diagnostic::Diagnostic;
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
    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    // Get the variable name or function name from a given expression. This will be used to
    // print out the message for this lint.
    fn get_var_name_or_func_name_from_exp(
        &mut self,
        exp: &ExpData,
        func_env: &FunctionEnv,
        env: &GlobalEnv,
    ) -> Option<String> {
        match exp {
            ExpData::Temporary(_, index) => {
                let parameters = func_env.get_parameters();
                let param = get_var_info_from_func_param(*index, &parameters).unwrap();
                Some(env.symbol_pool().string(param.0).to_string())
            },
            ExpData::LocalVar(_, sym) => Some(env.symbol_pool().string(*sym).to_string()),
            ExpData::Call(_, Operation::MoveFunction(module_id, func_id), _) => {
                let module = env.get_module(*module_id);
                let func_env = module.get_function(*func_id);
                let func_name = func_env
                    .get_name()
                    .display(func_env.symbol_pool())
                    .to_string();
                Some(func_name)
            },
            _ => None,
        }
    }

    // This function examines the provided expression to identify if it contains a comparison
    // between a boolean value and a variable (either a temporary or a local variable). If such
    // a comparison is found, it generates a diagnostic message suggesting a more direct
    // expression. For example, instead of `x == true`, it suggests using `x` directly, and
    // for `x == false`, it suggests using `!x`. The diagnostic message is emitted as a warning
    // to guide the user towards more idiomatic usage of boolean comparisons.
    fn check_boolean_comparison(
        &mut self,
        cond: &ExpData,
        func_env: &FunctionEnv,
        env: &GlobalEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        if let ExpData::Call(_, oper, args) = &cond {
            if let (Some(first_arg), Some(second_arg)) = (args.get(0), args.get(1)) {
                if let (
                    ExpData::Value(_, Value::Bool(a)),
                    ExpData::Temporary(_, _) | ExpData::LocalVar(_, _),
                ) = (first_arg.as_ref(), second_arg.as_ref())
                {
                    let var_name = self
                        .get_var_name_or_func_name_from_exp(second_arg, func_env, env)
                        .expect("Expected to get a variable name");

                    let diagnostic_msg = match (oper, a) {
                        (Operation::Eq, true) | (Operation::Neq, false) => Some(format!(
                            "Use {} directly instead of comparing it to {}.",
                            var_name, a
                        )),
                        (Operation::Eq, false) | (Operation::Neq, true) => Some(format!(
                            "Use !{} directly instead of comparing it to {}.",
                            var_name, a
                        )),
                        _ => None,
                    };

                    if let Some(diagnostic_msg) = diagnostic_msg {
                        add_diagnostic_and_emit(
                            &env.get_node_loc(cond.node_id()),
                            &diagnostic_msg,
                            codespan_reporting::diagnostic::Severity::Warning,
                            env,
                            diags,
                        );
                    }
                } else {
                    for exp in args {
                        self.check_boolean_comparison(exp, func_env, env, diags);
                    }
                }
            }

            if let Some(exp_val) = args.get(1) {
                if let ExpData::Value(_, Value::Bool(b)) = &exp_val.as_ref() {
                    let var_name = self
                        .get_var_name_or_func_name_from_exp(&args[0], func_env, env)
                        .expect("Expected to get a variable name");

                    let diagnostic_msg = match (oper, b) {
                        (Operation::Eq, true) | (Operation::Neq, false) => Some(format!(
                            "Use {} directly instead of comparing it to {}.",
                            var_name, b
                        )),
                        (Operation::Eq, false) | (Operation::Neq, true) => Some(format!(
                            "Use !{} directly instead of comparing it to {}.",
                            var_name, b
                        )),
                        _ => None,
                    };

                    if let Some(diagnostic_msg) = diagnostic_msg {
                        add_diagnostic_and_emit(
                            &env.get_node_loc(cond.node_id()),
                            &diagnostic_msg,
                            codespan_reporting::diagnostic::Severity::Warning,
                            env,
                            diags,
                        );
                    }
                } else {
                    for exp in args {
                        self.check_boolean_comparison(exp, func_env, env, diags);
                    }
                }
            }
        }
    }
}

impl ExpressionAnalysisVisitor for BoolComparisonVisitor {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        func_env: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        if let ExpData::IfElse(_, cond, _, _) = exp {
            self.check_boolean_comparison(cond.as_ref(), func_env, env, diags);
        }
    }
}
