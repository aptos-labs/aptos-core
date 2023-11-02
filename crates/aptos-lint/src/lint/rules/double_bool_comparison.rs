// Double comparisons occur when a value is compared twice with different relational operators
// inside a logical OR operation. For example, expressions like `a == b || a < b` or `x != y || x > y`.
// These patterns are potentially confusing and can be simplified for readability and maintainability.
use move_compiler::parser::ast::BinOp_;
use move_compiler::typing::ast::Exp;
use move_compiler::typing::ast as AST;

use crate::lint::context::VisitorContext;
use crate::lint::visitor::{LintVisitor, LintUtilities};

pub struct DoubleComparisonsVisitor;

impl DoubleComparisonsVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn LintVisitor> {
        Box::new(Self::new())
    }

    fn check_double_comparison(&mut self, exp: &Exp, context: &mut VisitorContext) {
        if let AST::UnannotatedExp_::BinopExp(left, op1, _, right) = &exp.exp.value {
            if op1.value == BinOp_::Or {
                if let AST::UnannotatedExp_::BinopExp(left2, op2, _, right2) = &right.exp.value {
                    if let AST::UnannotatedExp_::BinopExp(left3, op3, _, right3) = &left.exp.value {
                        if left3 == left2 && right3 == right2 {
                            match (op3.value, op2.value) {
                                // Check for cases where a value is checked for equality and then for less than,
                                (BinOp_::Eq, BinOp_::Lt) | (BinOp_::Lt, BinOp_::Eq) => {
                                    self.add_warning(
                                        context,
                                        &exp.exp.loc,
                                        &format!("Simplify double comparisons, use <= instead.")
                                    );
                                }
                                // Check for cases where a value is checked for equality and then for greater than,
                                (BinOp_::Eq, BinOp_::Gt) | (BinOp_::Gt, BinOp_::Eq) => {
                                    self.add_warning(
                                        context,
                                        &exp.exp.loc,
                                        &format!("Simplify double comparisons, use >= instead.")
                                    );
                                }
                                // Check for cases where a value is checked for inequality and then for less than,
                                (BinOp_::Neq, BinOp_::Lt) | (BinOp_::Lt, BinOp_::Neq) => {
                                    self.add_warning(
                                        context,
                                        &exp.exp.loc,
                                        &format!("Simplify double comparisons, use <= instead.")
                                    );
                                }
                                // Check for cases where a value is checked for inequality and then for greater than,
                                (BinOp_::Neq, BinOp_::Gt) | (BinOp_::Gt, BinOp_::Neq) => {
                                    self.add_warning(
                                        context,
                                        &exp.exp.loc,
                                        &format!("Simplify double comparisons, use >= instead.")
                                    );
                                }
                                _ => (),
                            }
                        }
                    }
                }
            }
        }
    }
}

impl LintVisitor for DoubleComparisonsVisitor {
    fn visit_exp(&mut self, exp: &Exp, context: &mut VisitorContext) {
        match &exp.exp.value {
            AST::UnannotatedExp_::IfElse(e1, _, _) | AST::UnannotatedExp_::While(e1, _) => {
                self.check_double_comparison(e1, context);
            }
            _ => {}
        }
    }
}
impl LintUtilities for DoubleComparisonsVisitor {}
