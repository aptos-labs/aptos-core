use codespan::FileId;
use codespan_reporting::diagnostic::Diagnostic;
use move_model::{
    ast::ExpData,
    model::{FunctionEnv, GlobalEnv, ModuleEnv},
};

use super::utils::LintConfig;

pub trait ExpressionAnalysisVisitor {
    /// Visit a module environment.
    /// Implement this method to define custom analysis logic for a module.
    fn visit_module(
        &mut self,
        _module: &ModuleEnv,
        _env: &GlobalEnv,
        _diags: &mut Vec<Diagnostic<FileId>>,
    ) {
    }

    /// Visit a function environment.
    /// Implement this method to define custom analysis logic for a function.
    fn visit_function(
        &mut self,
        func_env: &FunctionEnv,
        env: &GlobalEnv,
        lint_config: &LintConfig,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        if let Some(func) = func_env.get_def().as_ref() {
            func.visit_pre_post(&mut |is_pre_visit, exp: &ExpData| {
                if is_pre_visit {
                    self.pre_visit_expression(exp, func_env, env, lint_config, diags);
                } else {
                    self.post_visit_expression(exp, func_env, env, lint_config, diags);
                }
            });
        }
    }

    /// Visit a function environment.
    /// Implement this method to define custom analysis logic for a function.
    fn visit_function_custom(
        &mut self,
        _func_env: &FunctionEnv,
        _env: &GlobalEnv,
        _lint_config: &LintConfig,
        _diags: &mut Vec<Diagnostic<FileId>>,
    ) {
    }

    /// Visit a function with bytecode attached.
    /// Implement this method to define custom analysis logic for a function.
    fn visit_function_with_bytecode(
        &mut self,
        _func_env: &FunctionEnv,
        _env: &GlobalEnv,
        _diags: &mut Vec<Diagnostic<FileId>>,
    ) {
    }

    /// Checks to perform before visiting an expression.
    /// Implement this method to define behavior before an expression visit.
    fn pre_visit_expression(
        &mut self,
        _exp: &ExpData,
        _func_env: &FunctionEnv,
        _env: &GlobalEnv,
        _lint_config: &LintConfig,
        _diags: &mut Vec<Diagnostic<FileId>>,
    ) {
    }

    /// Checks to perform after visiting an expression.
    /// Implement this method to define behavior after an expression visit.
    fn post_visit_expression(
        &mut self,
        _exp: &ExpData,
        _func_env: &FunctionEnv,
        _env: &GlobalEnv,
        _lint_config: &LintConfig,
        _diags: &mut Vec<Diagnostic<FileId>>,
    ) {
    }

    fn requires_bytecode_inspection(&self) -> bool {
        false
    }
}
