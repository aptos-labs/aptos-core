// This linting rule detects expressions where multiplication appears before division.
// Such patterns can potentially affect the precision of the result, and this lint warns
// developers to ensure division operations precede multiplication in mathematical expressions.
use move_model::ast::{ Exp, ExpData, Operation };
use move_model::model::{ GlobalEnv, FunctionEnv };
use crate::lint::visitor::{ ExpDataVisitor, LintUtilities };

pub struct MultiplicationBeforeDivisionVisitor;

impl MultiplicationBeforeDivisionVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpDataVisitor> {
        Box::new(Self::new())
    }

    fn check_multiplication_before_division(&mut self, exp: &ExpData,  env: &GlobalEnv) {
        match &exp {
            ExpData::Block(_, _, exp_op, _) => {
                if let Some(call) = exp_op {
                    match call.as_ref() {
                        ExpData::Call(_, op, exp_vec) => {
                            if let Operation::Mul = op {
                                if self.has_binop_div_in_exp(&exp_vec[0]) {
                                    let message = &format!("Multiplication should come before division to avoid large rounding errors.");
                                    self.add_diagnostic_and_emit(
                                        &env.get_node_loc(call.node_id()),
                                        message,
                                        codespan_reporting::diagnostic::Severity::Warning,
                                        env
                                    );
            
                                }
                            }
                        }
                        _ => (),
                    }
                }
 
            }
            _ => (),
        }
    }

    fn has_binop_div_in_exp(&self, exp: &Exp) -> bool {
        match exp.as_ref() {
            ExpData::Call(_, op, e2) => {
                match op {
                    Operation::Div => true,
                    _ => self.has_binop_div_in_exp(&e2[0]),
                }
            }
            _ => false,
        }
    }
}

impl ExpDataVisitor for MultiplicationBeforeDivisionVisitor {
    fn visit(&mut self,  func_env: &FunctionEnv, env: &GlobalEnv) {
        let mut visitor = |exp: &ExpData| {
            self.check_multiplication_before_division(exp, env);
        };
        func_env.get_def().map(|func| {
            func.visit(&mut visitor);
        });
    }
}

impl LintUtilities for MultiplicationBeforeDivisionVisitor {}
