//! The lint identifies and warns about functions and structs that exceed the allowed limit of 32 type parameters.
//! This lint is useful for identifying functions and structs that may be overly complex and difficult to maintain.
//! By detecting these patterns, the lint encourages cleaner and more efficient code by suggesting the removal of unnecessary type parameters.
//! This lint enhances code quality by focusing on eliminating operations that have no effect, thereby improving readability and maintainability.
use crate::lint::{utils::add_diagnostic_and_emit, visitor::ExpressionAnalysisVisitor};
use codespan::FileId;

use codespan_reporting::diagnostic::Diagnostic;
use move_model::{
    ast::{Exp, ExpData, Operation},
    model::{FieldEnv, FieldId, FunctionEnv, GlobalEnv, ModuleEnv, ModuleId, StructId, Visibility},
    ty,
};
#[derive(Debug)]
pub struct ExceedParamsVisitor;

impl Default for ExceedParamsVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ExceedParamsVisitor {
    fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn check_function_and_struct(
        &self,
        module_env: &ModuleEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        for func_env in module_env.get_functions() {
            let type_params_count = func_env.get_type_parameter_count();
            if type_params_count > 32 {
                let message = format!(
                    "Function `{}` exceeds the allowed limit of 32 type parameters.",
                    func_env.get_name().display(module_env.symbol_pool())
                );
                add_diagnostic_and_emit(
                    &func_env.get_loc(),
                    &message,
                    codespan_reporting::diagnostic::Severity::Warning,
                    module_env.env,
                    diags,
                );
            }
        }

        for struct_env in module_env.get_structs() {
            let type_params_count = struct_env.get_type_parameters().len();
            if type_params_count > 32 {
                let message = format!(
                    "Struct `{}` exceeds the allowed limit of 32 type parameters.",
                    struct_env.get_name().display(module_env.symbol_pool())
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

impl ExpressionAnalysisVisitor for ExceedParamsVisitor {
    fn visit_module(
        &mut self,
        module: &ModuleEnv,
        _env: &GlobalEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        self.check_function_and_struct(module, diags);
    }
}
