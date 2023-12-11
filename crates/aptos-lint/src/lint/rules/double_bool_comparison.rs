// Double comparisons occur when a value is compared twice with different relational operators
// inside a logical OR operation. For example, expressions like `a == b || a < b` or `x != y || x > y`.
// These patterns are potentially confusing and can be simplified for readability and maintainability.
use move_model::ast::{ ExpData, Operation };
use move_model::model::{ GlobalEnv, FunctionEnv };
use crate::lint::visitor::{ LintUtilities, ExpDataVisitor };

pub struct DoubleComparisonsVisitor;

impl DoubleComparisonsVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpDataVisitor> {
        Box::new(Self::new())
    }

    fn check_double_comparison(&mut self, exp: &ExpData, env: &GlobalEnv) {
        match exp {
            ExpData::Call(_, oper, vec_exp) => {
                match oper {
                    Operation::Or => {
                        if let ExpData::Call(_, op1, _) = &vec_exp[0].as_ref() {
                            if let ExpData::Call(_, op2, _) = &vec_exp[1].as_ref() {
                                let left = &mut vec_exp[0].used_temporaries(env);
                                let right = &mut vec_exp[1].used_temporaries(env);
                                left.sort();
                                right.sort();
                                if left == right {
                                    match (op1, op2) {
                                        (Operation::Eq, Operation::Lt) | (Operation::Lt, Operation::Eq) => {
                                            self.add_diagnostic_and_emit(
                                                &env.get_node_loc(exp.node_id()),
                                                &format!("Simplify double comparisons, use <= instead."),
                                                codespan_reporting::diagnostic::Severity::Warning,
                                                env
                                            );
                                        }
                                        (Operation::Eq, Operation::Gt) | (Operation::Gt, Operation::Eq) => {
                                            self.add_diagnostic_and_emit(
                                                &env.get_node_loc(exp.node_id()),
                                                &format!("Simplify double comparisons, use >= instead."),
                                                codespan_reporting::diagnostic::Severity::Warning,
                                                env
                                            );
                                        }
                                        (Operation::Neq, Operation::Lt) | (Operation::Lt, Operation::Neq) => {
                                            self.add_diagnostic_and_emit(
                                                &env.get_node_loc(exp.node_id()),
                                                &format!("Simplify double comparisons, use < instead."),
                                                codespan_reporting::diagnostic::Severity::Warning,
                                                env
                                            );
                                        }
                                        (Operation::Neq, Operation::Gt) | (Operation::Gt, Operation::Neq) => {
                                            self.add_diagnostic_and_emit(
                                                &env.get_node_loc(exp.node_id()),
                                                &format!("Simplify double comparisons, use > instead."),
                                                codespan_reporting::diagnostic::Severity::Warning,
                                                env
                                            );
                                        }
                                        _ => (),
                                    }
                                }
                            }
                        }
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    }
}

impl ExpDataVisitor for DoubleComparisonsVisitor {
    fn visit(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        let mut visitor = |t: &ExpData| {
            match t {
                ExpData::IfElse(_, e1, _, _) => {
                    self.check_double_comparison(e1.as_ref(), env);
                }
                _ => (),
            }
        };
        func_env
            .get_def()
            .map(|func| {
                func.visit(&mut visitor);
            });
    }
}
impl LintUtilities for DoubleComparisonsVisitor {}
