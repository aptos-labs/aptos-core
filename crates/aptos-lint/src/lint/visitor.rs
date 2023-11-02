use move_compiler::typing::ast::{ Function, Sequence, Exp, ModuleDefinition };
use move_compiler::typing::ast as AST;

use super::context::VisitorContext;

pub trait LintVisitor {
    fn visit_module(&mut self, _module: &ModuleDefinition, _context: &mut VisitorContext) {}

    fn visit_function(&mut self, function: &Function, context: &mut VisitorContext) {
        match &function.body.value {
            AST::FunctionBody_::Defined(block) => {
                self.visit_sequence(block, context);
            }
            _ => (),
        }
    }

    fn visit_sequence(&mut self, block: &Sequence, context: &mut VisitorContext) {
        for seq in block {
            match &seq.value {
                AST::SequenceItem_::Seq(exp) => {
                    self.visit_exp(exp, context);
                }
                // AST::SequenceItem_::Declare(exp) => {
                //     self.visit_exp(exp, context);
                // }
                AST::SequenceItem_::Bind(_, _, exp) => {
                    self.visit_exp(exp, context);
                }
                _ => {}
            }
        }
    }

    fn visit_exp(&mut self, _function: &Exp, _context: &mut VisitorContext) {}
}

pub trait LintUtilities {
    fn normalize_binary_op(&self, left: &str, op: &str, right: &str) -> String {
        match op {
            "Eq" | "Neq" => {
                let (l, r) = if left <= right { (left, right) } else { (right, left) };
                format!("{} {} {}", l, op, r)
            }
            "Gt" => { format!("{} Lt {}", right, left) }
            "Ge" => { format!("{} Le {}", right, left) }
            "Lt" => { format!("{} Gt {}", right, left) }
            "Le" => { format!("{} Ge {}", right, left) }
            _ => format!("{} {} {}", left, op, right),
        }
    }

    fn get_condition_string(&self, exp: &Box<Exp>) -> String {
        match &exp.exp.value {
            AST::UnannotatedExp_::BinopExp(left, op, _, right) => {
                let left_str = self.get_exp_string(left);
                let op_str = format!("{:?}", op);
                let right_str = self.get_exp_string(right);
                self.normalize_binary_op(&left_str, &op_str, &right_str)
            }
            AST::UnannotatedExp_::Copy { var, .. } => var.to_string(),
            AST::UnannotatedExp_::Value(val) => format!("{:?}", val),
            _ => "".to_string(),
        }
    }

    fn get_exp_string(&self, e: &Box<Exp>) -> String {
        let condition_string = self.get_condition_string(e);
        condition_string
    }

    fn add_diagnostic_with_severity(
        &mut self,
        context: &mut VisitorContext,
        loc: &move_ir_types::location::Loc,
        message: &str,
        severity: codespan_reporting::diagnostic::Severity
    ) {
        if let Some(f) = context.ast.files.get(&loc.file_hash()) {
            let file_id = context.add_file(f.0.to_string(), f.1.clone().into());
            context.add_diagnostic(file_id, loc.start() as usize, loc.end() as usize, message, severity);
        }
    }

    fn add_warning(&mut self, context: &mut VisitorContext, loc: &move_ir_types::location::Loc, message: &str) {
        self.add_diagnostic_with_severity(context, loc, message, codespan_reporting::diagnostic::Severity::Warning);
    }

    fn add_error(&mut self, context: &mut VisitorContext, loc: &move_ir_types::location::Loc, message: &str) {
        self.add_diagnostic_with_severity(context, loc, message, codespan_reporting::diagnostic::Severity::Error);
    }
}
