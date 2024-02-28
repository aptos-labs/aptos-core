//! Check for functions that take mutable references but don't actually mutate anything.
use crate::lint::{
    utils::{add_diagnostic_and_emit, get_var_info_from_func_param},
    visitor::ExpressionAnalysisVisitor,
};
use codespan::FileId;

use codespan_reporting::diagnostic::Diagnostic;
use move_model::{
    ast::ExpData,
    model::{FunctionEnv, GlobalEnv, Parameter},
};
#[derive(Debug)]
pub struct UnmodifiedMutableArgumentLint;

impl Default for UnmodifiedMutableArgumentLint {
    fn default() -> Self {
        Self::new()
    }
}

impl UnmodifiedMutableArgumentLint {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn check_unmodified_mut_arguments(
        &self,
        func_env: &FunctionEnv,
        env: &GlobalEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        for param in func_env.get_parameters().iter() {
            if param.1.is_mutable_reference() && !self.is_argument_modified(param, func_env) {
                let message = format!(
                    "Mutable parameter '{}' is never modified in function '{}'.",
                    param.0.display(func_env.symbol_pool()),
                    func_env.get_name().display(func_env.symbol_pool())
                );
                add_diagnostic_and_emit(
                    &func_env.get_loc(),
                    &message,
                    codespan_reporting::diagnostic::Severity::Warning,
                    env,
                    diags,
                );
            }
        }
    }

    fn is_argument_modified(&self, param: &Parameter, func_env: &FunctionEnv) -> bool {
        let param_name = param.0.display(func_env.symbol_pool()).to_string();
        let mut used = false;
        if let Some(func_body) = func_env.get_def().as_ref() {
            func_body.visit_pre_post(
                &mut (|up: bool, exp: &ExpData| {
                    if !up && !used {
                        if let ExpData::Mutate(_, lhs, _) = exp {
                            if let ExpData::Call(_, _, vec_exp) = lhs.as_ref() {
                                for exp in vec_exp {
                                    if let ExpData::Temporary(_, index) = exp.as_ref() {
                                        if let Some(param) = get_var_info_from_func_param(
                                            *index,
                                            &func_env.get_parameters(),
                                        ) {
                                            if param.0.display(func_env.symbol_pool()).to_string()
                                                == param_name
                                            {
                                                used = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }),
            );
        }
        used
    }
}

impl ExpressionAnalysisVisitor for UnmodifiedMutableArgumentLint {
    fn visit_module(
        &mut self,
        _module: &move_model::model::ModuleEnv,
        _env: &GlobalEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        for func_env in _module.get_functions() {
            self.check_unmodified_mut_arguments(&func_env, _env, diags);
        }
    }
}
