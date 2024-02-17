use move_model::{
    ast::ExpData,
    model::{FunctionEnv, GlobalEnv, ModuleEnv},
};
use move_stackless_bytecode::function_target_pipeline::FunctionTargetsHolder;

use super::utils::LintConfig;

pub trait ExpressionAnalysisVisitor {
    /// Visit a module environment.
    /// Implement this method to define custom analysis logic for a module.
    fn visit_module(&mut self, _module: &ModuleEnv, _env: &GlobalEnv) {}

    /// Visit a function environment.
    /// Implement this method to define custom analysis logic for a function.
    fn visit_function(
        &mut self,
        func_env: &FunctionEnv,
        env: &GlobalEnv,
        lint_config: &LintConfig,
    ) {
        if let Some(func) = func_env.get_def() {
            func.visit_pre_post(&mut |is_pre_visit, exp: &ExpData| {
                if is_pre_visit {
                    self.pre_visit_expression(exp, func_env, env, lint_config);
                } else {
                    self.post_visit_expression(exp, func_env, env, lint_config);
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
    ) {
    }

    /// Visit a function with bytecode attached.
    /// Implement this method to define custom analysis logic for a function.
    fn visit_function_with_bytecode(&mut self, _func_env: &FunctionEnv, _env: &GlobalEnv) {}

    /// Checks to perform before visiting an expression.
    /// Implement this method to define behavior before an expression visit.
    fn pre_visit_expression(
        &mut self,
        _exp: &ExpData,
        _func_env: &FunctionEnv,
        _env: &GlobalEnv,
        _lint_config: &LintConfig,
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
    ) {
    }

    fn requires_bytecode_inspection(&self) -> bool {
        false
    }
}
