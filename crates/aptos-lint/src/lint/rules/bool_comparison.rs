// Detects comparisons where a variable is compared to 'true' or 'false' using
// equality (==) or inequality (!=) operators and provides suggestions to simplify the comparisons.
use move_compiler::expansion::ast::Value_;
use move_compiler::parser::ast::BinOp_;
use move_compiler::typing::ast::Exp;
use move_compiler::typing::ast as AST;

use crate::lint::context::VisitorContext;
use crate::lint::visitor::{LintVisitor, LintUtilities};

pub struct BoolComparisonVisitor;
impl BoolComparisonVisitor {
    pub fn new() -> Self {
        Self {}
    }
    pub fn visitor() -> Box<dyn LintVisitor> {
        Box::new(Self::new())
    }

    fn check_boolean_comparison(&mut self, exp: &Exp, context: &mut VisitorContext) {
        if let AST::UnannotatedExp_::BinopExp(left, op, _, right) = &exp.exp.value {
            if let AST::UnannotatedExp_::Value(val) = &right.exp.value {
                if let Value_::Bool(b) = val.value {
                    let variable = self.get_exp_string(left);
                    match (op.value, b) {
                        // Checking for comparisons where a variable is compared to 'true' using equality
                        // or to 'false' using inequality. These can be simplified by using the variable directly.
                        (BinOp_::Eq, true) | (BinOp_::Neq, false) => {
                            self.add_warning(
                                context,
                                &exp.exp.loc,
                                &format!("Use {} directly instead of comparing it to {}.", variable, b)
                            );
                        }
                        // Checking for comparisons where a variable is compared to 'false' using equality
                        // or to 'true' using inequality. These can be simplified by negating the variable.
                        (BinOp_::Eq, false) | (BinOp_::Neq, true) => {
                            self.add_warning(
                                context,
                                &exp.exp.loc,
                                &format!("Use !{} instead of comparing it to {}.", variable, b)
                            );
                        }
                        _ => (),
                    }
                }
            }
        }
    }
}

impl LintVisitor for BoolComparisonVisitor {
    fn visit_exp(&mut self, exp: &Exp, context: &mut VisitorContext) {
        match &exp.exp.value {
            AST::UnannotatedExp_::IfElse(e1, _, _) | AST::UnannotatedExp_::While(e1, _) => {
                self.check_boolean_comparison(e1, context);
            }
            _ => (),
        }

    }
}

impl LintUtilities for BoolComparisonVisitor {}
