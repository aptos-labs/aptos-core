//! Detects comparisons where a variable or value returned from function is compared to 'true' or 'false' using
//! equality (==) or inequality (!=) operators and provides suggestions to simplify the comparisons.
//! Examples: if (x == true) can be simplified to if (x), if (x == false) can be simplified to if (!x)
use crate::lint::{
    utils::{add_diagnostic_and_emit, get_var_name_or_func_name_from_exp, LintConfig},
    visitor::ExpressionAnalysisVisitor,
};
use codespan::FileId;
use codespan_reporting::diagnostic::Diagnostic;
use move_model::{
    ast::{ExpData, Operation, Value},
    model::{FunctionEnv, GlobalEnv},
};
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
                    let var_name = get_var_name_or_func_name_from_exp(second_arg, func_env, env)
                        .expect("Expected to get a variable name");

                    let diagnostic_msg = match (oper, a) {
                        (Operation::Eq, true) | (Operation::Neq, false) => Some(format!(
                            "Use `{}` directly instead of comparing it to `{}`.",
                            var_name, a
                        )),
                        (Operation::Eq, false) | (Operation::Neq, true) => Some(format!(
                            "Use `!{}` directly instead of qcomparing it to `{}`.",
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
                    if let (
                        ExpData::Call(_, Operation::Not, neg_args),
                        ExpData::Value(_, Value::Bool(b)),
                    ) = (first_arg.as_ref(), second_arg.as_ref())
                    {
                        if !*b {
                            // Check if the comparison is with false
                            if let Some(neg_var) = neg_args.get(0) {
                                let var_name =
                                    get_var_name_or_func_name_from_exp(neg_var, func_env, env)
                                        .expect("Expected to get a variable name");

                                let diagnostic_msg = format!(
                                    "Use `{}` directly instead of `!{} == false`.",
                                    var_name, var_name
                                );

                                add_diagnostic_and_emit(
                                    &env.get_node_loc(cond.node_id()),
                                    &diagnostic_msg,
                                    codespan_reporting::diagnostic::Severity::Warning,
                                    env,
                                    diags,
                                );
                            }
                        }
                    }
                }
                for exp in args {
                    self.check_boolean_comparison(exp, func_env, env, diags);
                }
            }
            if args.len() == 2 {
                let (first_arg, second_arg) = (&args[0], &args[1]);
                // Determine if one of the arguments is a boolean literal `true` and the other is a call or variable
                let is_comparison_with_true =
                    |arg: &ExpData| matches!(arg, ExpData::Value(_, Value::Bool(true)));
                let is_function_or_variable = |arg: &ExpData| {
                    matches!(
                        arg,
                        ExpData::Call(_, Operation::MoveFunction(_, _), _)
                            | ExpData::Temporary(_, _)
                            | ExpData::LocalVar(_, _)
                    )
                };

                if is_comparison_with_true(first_arg.as_ref())
                    || is_comparison_with_true(second_arg.as_ref())
                {
                    let (_, other_arg) = if is_comparison_with_true(first_arg.as_ref()) {
                        (first_arg, second_arg)
                    } else {
                        (second_arg, first_arg)
                    };
                    if is_function_or_variable(other_arg.as_ref()) {
                        let var_name = get_var_name_or_func_name_from_exp(other_arg, func_env, env)
                            .expect("Expected to get a variable or function name");
                        let var_type = if matches!(
                            other_arg.as_ref(),
                            ExpData::Call(_, Operation::MoveFunction(_, _), _)
                        ) {
                            "function"
                        } else {
                            "variable"
                        };
                        let use_directly = var_type == "variable" || oper == &Operation::Eq;
                        let diagnostic_msg: String;

                        if var_type == "variable" {
                            diagnostic_msg = if use_directly {
                                format!(
                                    "Use `{}` directly instead of comparing it to true.",
                                    var_name
                                )
                            } else {
                                // Implies oper == &Operation::Neq due to prior condition
                                format!(
                                    "Use `!{}` directly instead of comparing it to true.",
                                    var_name
                                )
                            };
                        } else if var_type == "function" {
                            diagnostic_msg = if oper == &Operation::Eq {
                                format!(
                                    "Call `{}` directly instead of comparing it to true.",
                                    var_name
                                )
                            } else {
                                // Implies oper == &Operation::Neq
                                format!(
                                    "Call `!{}` directly instead of comparing it to true.",
                                    var_name
                                )
                            };
                        } else {
                            diagnostic_msg = format!(
                                "Consider simplifying the comparison involving `{}`.",
                                var_name
                            );
                        }

                        add_diagnostic_and_emit(
                            &env.get_node_loc(cond.node_id()),
                            &diagnostic_msg,
                            codespan_reporting::diagnostic::Severity::Warning,
                            env,
                            diags,
                        );
                    }
                }
            }
            args.iter()
                .for_each(|exp| self.check_boolean_comparison(exp, func_env, env, diags));
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
        } else {
            self.check_boolean_comparison(exp, func_env, env, diags);
        }
    }
}
