use crate::lint::{utils::add_diagnostic_and_emit, visitor::ExpressionAnalysisVisitor};
use move_model::{
    ast::{Exp, ExpData, Operation},
    model::{FieldEnv, FieldId, FunctionEnv, GlobalEnv, ModuleEnv, ModuleId, StructId, Visibility},
};

#[derive(Debug)]
pub struct GetterMethodFieldMatchLint;

impl Default for GetterMethodFieldMatchLint {
    fn default() -> Self {
        Self::new()
    }
}

impl GetterMethodFieldMatchLint {
    fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn check_getter_methods(&self, module_env: &ModuleEnv) {
        for func_env in module_env.get_functions() {
            if !self.is_getter_method(&func_env) {
                continue;
            }

            let method_name = func_env
                .get_name()
                .display(module_env.symbol_pool())
                .to_string();
            if let Some(func) = func_env.get_def() {
                self.check_function_definition(func, &func_env, &method_name, module_env);
            }
        }
    }

    fn is_getter_method(&self, func_env: &FunctionEnv) -> bool {
        func_env.visibility() == Visibility::Public && func_env.get_parameters().len() <= 1
    }

    fn check_function_definition(
        &self,
        func: &Exp,
        func_env: &FunctionEnv,
        method_name: &str,
        module_env: &ModuleEnv,
    ) {
        func.visit_pre_post(&mut |up, exp| {
            if !up {
                if let ExpData::Return(_, return_exp) = exp {
                    self.check_return_expression(return_exp, func_env, method_name, module_env);
                }
            }
        });
    }

    fn check_return_expression(
        &self,
        return_exp: &ExpData,
        func_env: &FunctionEnv,
        method_name: &str,
        module_env: &ModuleEnv,
    ) {
        if let ExpData::Call(_, _, _) = return_exp {
            return_exp.visit_pre_post(&mut |up, exp| {
                if !up {
                    self.process_expression(exp, func_env, method_name, module_env);
                }
            });
        } else {
            self.report_non_call_return(func_env, method_name, module_env);
        }
    }

    fn process_expression(
        &self,
        exp: &ExpData,
        func_env: &FunctionEnv,
        method_name: &str,
        module_env: &ModuleEnv,
    ) {
        if let ExpData::Call(_, Operation::Select(module_id, struct_id, field_id), _) = exp {
            self.process_select_operation(
                module_id,
                struct_id,
                field_id,
                func_env,
                method_name,
                module_env,
            );
        }
    }

    fn process_select_operation(
        &self,
        module_id: &ModuleId,
        struct_id: &StructId,
        field_id: &FieldId,
        func_env: &FunctionEnv,
        method_name: &str,
        module_env: &ModuleEnv,
    ) {
        let struct_env = func_env
            .module_env
            .env
            .get_struct(module_id.qualified(*struct_id));
        let field_env = struct_env.get_field(*field_id);

        let field_name = field_env
            .get_name()
            .display(module_env.symbol_pool())
            .to_string();
        let field_type = field_env.get_type();

        if !method_name.contains(&field_name) || field_type != func_env.get_result_type() {
            self.report_mismatched_getter(func_env, method_name, &field_env, module_env);
        }
    }

    fn report_mismatched_getter(
        &self,
        func_env: &FunctionEnv,
        method_name: &str,
        field_env: &FieldEnv,
        module_env: &ModuleEnv,
    ) {
        let message = format!(
            "Getter method '{}' returns a field '{}' which does not match its name.",
            method_name,
            field_env.get_name().display(module_env.symbol_pool())
        );
        add_diagnostic_and_emit(
            &module_env
                .env
                .get_node_loc(func_env.get_def().unwrap().node_id()),
            &message,
            codespan_reporting::diagnostic::Severity::Warning,
            module_env.env,
        );
    }

    fn report_non_call_return(
        &self,
        func_env: &FunctionEnv,
        method_name: &str,
        module_env: &ModuleEnv,
    ) {
        let message = format!(
            "Getter method '{}' does not return required field.",
            method_name
        );
        add_diagnostic_and_emit(
            &module_env
                .env
                .get_node_loc(func_env.get_def().unwrap().node_id()),
            &message,
            codespan_reporting::diagnostic::Severity::Warning,
            module_env.env,
        );
    }
}

impl ExpressionAnalysisVisitor for GetterMethodFieldMatchLint {
    fn visit_module(&mut self, _module: &ModuleEnv, _env: &GlobalEnv) {
        self.check_getter_methods(_module);
    }
}
