//! The lint identifies and warns about modules that exceed the allowed limit of structs, fields, and functions.
//! This lint is useful for identifying modules that may be overly complex and difficult to maintain.
use crate::lint::{utils::add_diagnostic_and_emit, visitor::ExpressionAnalysisVisitor};
use codespan::FileId;

use codespan_reporting::diagnostic::Diagnostic;
use move_bytecode_verifier::VerifierConfig;
use move_model::{
    ast::{Exp, ExpData, Operation},
    model::{FieldEnv, FieldId, FunctionEnv, GlobalEnv, ModuleEnv, ModuleId, StructId, Visibility},
    ty,
};
#[derive(Debug)]
pub struct ExceedFieldsVisitor;

impl Default for ExceedFieldsVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ExceedFieldsVisitor {
    fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn check_exceed_fields_and_functions(
        &self,
        module_env: &ModuleEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        let config = VerifierConfig::production();

        let struct_count: usize = module_env.get_struct_count();
        if let Some(max_struct) = config.max_struct_definitions {
            if struct_count > max_struct {
                let message = format!(
                    "Module `{}` exceeds the allowed limit of {} structs.",
                    module_env.get_name().display(module_env.env),
                    max_struct
                );
                add_diagnostic_and_emit(
                    &module_env.get_loc(),
                    &message,
                    codespan_reporting::diagnostic::Severity::Warning,
                    module_env.env,
                    diags,
                );
            }
        }

        let function_count = module_env.get_function_count();
        if let Some(max_function) = config.max_function_definitions {
            if function_count > max_function {
                let message = format!(
                    "Module `{}` exceeds the allowed limit of {} functions.",
                    module_env.get_name().display(module_env.env),
                    max_function
                );
                add_diagnostic_and_emit(
                    &module_env.get_loc(),
                    &message,
                    codespan_reporting::diagnostic::Severity::Warning,
                    module_env.env,
                    diags,
                );
            }
        }

        for struct_env in module_env.get_structs() {
            let field_count = struct_env.get_field_count();
            if let Some(max_field) = config.max_fields_in_struct {
                if field_count > max_field {
                    let message = format!(
                        "Struct `{}` exceeds the allowed limit of {} fields.",
                        struct_env.get_name().display(module_env.symbol_pool()),
                        max_field
                    );
                    add_diagnostic_and_emit(
                        &struct_env.get_loc(),
                        &message,
                        codespan_reporting::diagnostic::Severity::Warning,
                        module_env.env,
                        diags,
                    );
                }
            }
        }
    }
}

impl ExpressionAnalysisVisitor for ExceedFieldsVisitor {
    fn visit_module(
        &mut self,
        module: &ModuleEnv,
        _env: &GlobalEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        self.check_exceed_fields_and_functions(module, diags);
    }
}
