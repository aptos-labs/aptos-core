// Consecutive 'if' statements with identical conditions are usually redundant and can be refactored
// to improve code readability and maintainability.
use move_compiler::typing::ast::Exp;
use move_compiler::typing::ast as AST;

use crate::lint::context::VisitorContext;
use crate::lint::visitor::{LintVisitor, LintUtilities};

pub struct IfsSameCondVisitor {
    if_condition: Vec<String>,
}

impl IfsSameCondVisitor {
    pub fn new() -> Self {
        Self {
            if_condition: Vec::new(),
        }
    }
    pub fn visitor() -> Box<dyn LintVisitor> {
        Box::new(Self::new())
    }

    fn check_and_set_condition(&mut self, exp: &Box<Exp>, context: &mut VisitorContext) {
        let current_condition = self.get_condition_string(exp);

        if self.if_condition.iter().any(|e| current_condition.contains(e)) {
            self.add_warning(
                context,
                &exp.exp.loc,
                "Detected consecutive if conditions with the same expression. Consider refactoring to avoid redundancy."
            );
        } else {
            self.if_condition.push(current_condition);
        }
    }

    fn clear_condition(&mut self) {
        self.if_condition = Vec::new();
    }
}

impl LintVisitor for IfsSameCondVisitor {
    fn visit_exp(&mut self, exp: &Exp, context: &mut VisitorContext) {
        match &exp.exp.value {
            AST::UnannotatedExp_::IfElse(e1, _, e3) => {
                self.check_and_set_condition(e1, context);
                let mut next_exp = e3.as_ref();
                loop {
                    if let AST::UnannotatedExp_::IfElse(e1, _, e3) = &next_exp.exp.value {
                        self.check_and_set_condition(e1, context);
                        next_exp = e3.as_ref();
                    } else {
                        break;
                    }
                }
            }
            _ => {
                self.clear_condition();
            }
        }
    }
}

impl LintUtilities for IfsSameCondVisitor {}
