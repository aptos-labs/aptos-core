// This linting rule detects expressions where multiplication appears before division.
// Such patterns can potentially affect the precision of the result, and this lint warns
// developers to ensure division operations precede multiplication in mathematical expressions.
use move_compiler::typing::ast::{self as AST, Exp};
use move_compiler::parser::ast as AST1;
use crate::lint::context::VisitorContext;
use crate::lint::visitor::{LintVisitor, LintUtilities};

pub struct MultiplicationBeforeDivisionVisitor;

impl MultiplicationBeforeDivisionVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn LintVisitor> {
        Box::new(Self::new())
    }

    fn check_multiplication_before_division(&mut self, exp: &Exp, context: &mut VisitorContext) {
        match &exp.exp.value {
            AST::UnannotatedExp_::BinopExp(e1, op, _, _) => {
                if let AST1::BinOp_::Mul = &op.value {
                    if self.has_binop_div_in_exp(e1) {
                        let message = &format!("Multiplication should come before division to avoid large rounding errors.");
                        self.add_warning(context, &exp.exp.loc, message);

                    }
                }
            },
            _ => (),
        }
    }

    fn has_binop_div_in_exp(&self, exp: &Exp) -> bool {
        match &exp.exp.value {
            AST::UnannotatedExp_::BinopExp(e1, op, _, e2) => {
                match &op.value {
                    AST1::BinOp_::Div => true,
                    _ => self.has_binop_div_in_exp(e1) || self.has_binop_div_in_exp(e2),
                }
            }
            _ => false,
        }
    }
}

impl LintVisitor for MultiplicationBeforeDivisionVisitor {
    fn visit_exp(&mut self, exp: &Exp, context: &mut VisitorContext) {
        self.check_multiplication_before_division(exp, context);
    }
}

impl LintUtilities for MultiplicationBeforeDivisionVisitor {}
