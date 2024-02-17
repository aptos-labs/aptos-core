use super::{
    build::CompiledModel, rules::unused_borrow_global_mut::UnusedBorrowGlobalMutVisitor,
    utils::LintConfig, visitor::ExpressionAnalysisVisitor,
};

use move_model::model::{FunctionEnv, ModuleEnv};
use move_stackless_bytecode::function_target_pipeline::{FunctionTargetsHolder, FunctionVariant};

pub struct VisitorManager {
    linters: Vec<Box<dyn ExpressionAnalysisVisitor>>,
}

impl VisitorManager {
    pub fn new(linters: Vec<Box<dyn ExpressionAnalysisVisitor>>) -> Self {
        Self { linters }
    }

    pub fn run(&mut self, env: (CompiledModel, CompiledModel), lint_config: &LintConfig) {
        for module_env in env
            .0
            .model
            .get_target_modules()
            .iter()
            .zip(env.1.model.get_target_modules().iter())
        {
            self.apply_linters_to_module(module_env, lint_config);
        }
    }

    /// Applies all registered linters to a given module and its functions.
    fn apply_linters_to_module(
        &mut self,
        module_env: (&ModuleEnv, &ModuleEnv),
        lint_config: &LintConfig,
    ) {
        for linter in &mut self.linters {
            // Visit the module environment with the current linter.
            linter.visit_module(module_env.1, module_env.1.env);

            // Visit each function within the module environment with the current linter.
            for func_env in module_env.1.get_functions() {
                linter.visit_function_custom(&func_env, module_env.1.env, lint_config);
                linter.visit_function(&func_env, module_env.1.env, lint_config);
            }
            for func_env in module_env.0.get_functions() {
                if linter.requires_bytecode_inspection() {
                    linter.visit_function_with_bytecode(&func_env, module_env.0.env);
                }
            }
        }
    }
}
