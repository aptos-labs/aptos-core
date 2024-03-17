use super::{build::CompiledModel, utils::LintConfig, visitor::ExpressionAnalysisVisitor};
use codespan::FileId;
use codespan_reporting::diagnostic::Diagnostic;
use move_model::model::ModuleEnv;

pub struct VisitorManager {
    linters: Vec<Box<dyn ExpressionAnalysisVisitor>>,
    diagnostics: Vec<Diagnostic<FileId>>,
}

impl VisitorManager {
    pub fn new(linters: Vec<Box<dyn ExpressionAnalysisVisitor>>) -> Self {
        Self {
            linters,
            diagnostics: Vec::new(),
        }
    }

    /// Runs all registered linters on the given environment.
    /// This method will visit each module and function within the environment with each registered linter
    /// The env tuple contains 2 CompiledModel, one for the bytecode attched and one for the bytecode not attatched.
    /// The reason for this is that some linters require bytecode inspection and some don't.
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
            linter.visit_module(module_env.1, module_env.1.env, &mut self.diagnostics);

            // Visit each function within the module environment with the current linter.
            for func_env in module_env.1.get_functions() {
                linter.visit_function_custom(
                    &func_env,
                    module_env.1.env,
                    lint_config,
                    &mut self.diagnostics,
                );
                linter.visit_function(
                    &func_env,
                    module_env.1.env,
                    lint_config,
                    &mut self.diagnostics,
                );
            }
            for func_env in module_env.0.get_functions() {
                if linter.requires_bytecode_inspection() {
                    linter.visit_function_with_bytecode(
                        &func_env,
                        module_env.0.env,
                        &mut self.diagnostics,
                    );
                }
            }
        }
    }

    pub fn diagnostics(&self) -> Vec<Diagnostic<FileId>> {
        self.diagnostics.clone()
    }
}
