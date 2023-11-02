// The visitor specifically targets cases where a variable is being cast to the same type it already has.
// Such type conversions are redundant and can be omitted for cleaner and more readable code.
use move_compiler::naming::ast::{ Type_, TypeName_, BuiltinTypeName };
use move_compiler::typing::ast as AST;
use move_compiler::typing::ast::Exp;

use crate::lint::context::VisitorContext;
use crate::lint::visitor::{LintVisitor, LintUtilities};

pub struct UnnecessaryTypeConversionVisitor;

impl UnnecessaryTypeConversionVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn LintVisitor> {
        Box::new(Self::new())
    }

    fn extract_builtin_type_name(ty: &Type_) -> Option<&BuiltinTypeName> {
        if let Type_::Apply(_, type_name, _) = ty {
            if let TypeName_::Builtin(builtin_name) = &type_name.value { Some(builtin_name) } else { None }
        } else {
            None
        }
    }

    fn check_unnecessary_conversion(&mut self, exp: &Exp, context: &mut VisitorContext) {
        if let AST::UnannotatedExp_::Cast(e, typ) = &exp.exp.value {
            // Checking if an expression is a type cast operation.
            // If the original type and the target type of the cast operation are the same,
            // a warning is generated because such a cast is redundant.
            if &e.ty.value == &typ.value {
                let var_name = self.get_exp_string(e);
                let type_name = format!(
                    "{:?}",
                    UnnecessaryTypeConversionVisitor::extract_builtin_type_name(&typ.value).unwrap()
                );
                let message = &format!(
                    "Unnecessary type conversion detected. '{}' is already of type '{}'. Avoid casting it to its own type.",
                    var_name,
                    type_name
                );
                self.add_warning(context, &exp.exp.loc, message);
            }
        }
    }
}

impl LintVisitor for UnnecessaryTypeConversionVisitor {
    fn visit_exp(&mut self, exp: &Exp, context: &mut VisitorContext) {
        self.check_unnecessary_conversion(exp, context);
    }
}

impl LintUtilities for UnnecessaryTypeConversionVisitor {}
