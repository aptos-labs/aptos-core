//! Detect non-stdlib inline functions that have a high number of lines of code and are used
//! multiple times in the code. This can lead to a high bytecode size as the inline function
//! is expanded in each use. This lint defaults to 10 lines of code and 2 usages but these numbers
//! can be configured in the lint config.
use crate::lint::{
    utils::{add_diagnostic_and_emit, LintConfig},
    visitor::ExpressionAnalysisVisitor,
};
use move_model::{
    ast::ExpData,
    model::{FunctionEnv, GlobalEnv},
};

#[derive(Debug)]
pub struct ComplexInlineFunctionVisitor {
    statement_count: usize,
}

impl Default for ComplexInlineFunctionVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ComplexInlineFunctionVisitor {
    pub fn new() -> Self {
        Self { statement_count: 0 }
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    // Returns true if the given function has a lot of code and is used many times.
    fn is_complex_and_frequently_used(
        &mut self,
        func_env: &FunctionEnv,
        env: &GlobalEnv,
        lint_config: &LintConfig,
    ) -> bool {
        let is_inline = func_env.is_inline();
        let usage_frequency = self.get_usage_frequency(func_env);

        let not_defined_at_0x1 = !self.is_defined_at_0x1(func_env, env);

        is_inline
            && not_defined_at_0x1
            && self.statement_count >= lint_config.statement_count
            && usage_frequency > lint_config.usage_frequency
    }

    fn is_defined_at_0x1(&self, func_env: &FunctionEnv, env: &GlobalEnv) -> bool {
        func_env.module_env.self_address() == &env.get_stdlib_address()
    }

    fn count_calls(&mut self, exp: &ExpData) {
        if let ExpData::Call(_, _, _) = exp {
            self.statement_count += 1;
        }
    }

    fn get_usage_frequency(&self, func_env: &FunctionEnv) -> usize {
        if let Some(calling_functions) = func_env.get_calling_functions() {
            return calling_functions.len();
        }
        0
    }
}

impl ExpressionAnalysisVisitor for ComplexInlineFunctionVisitor {
    fn visit_function_custom(
        &mut self,
        func_env: &FunctionEnv,
        env: &GlobalEnv,
        lint_config: &LintConfig,
    ) {
        if let Some(func) = func_env.get_def() {
            func.visit_pre_post(
                &mut (|up: bool, exp: &ExpData| {
                    if !up {
                        self.count_calls(exp);
                    }
                }),
            );

            let is_complex_and_frequently_used =
                self.is_complex_and_frequently_used(func_env, env, lint_config);

            if is_complex_and_frequently_used {
                let message =
                    format!(
                        "Inline function is longer than {} lines and used {} times in this code. This would lead to a much larger bytecode as the inline function is expanded in each use",
                        lint_config.statement_count,
                        lint_config.usage_frequency,
                    );
                add_diagnostic_and_emit(
                    &func_env.get_loc(),
                    &message,
                    codespan_reporting::diagnostic::Severity::Warning,
                    env,
                );
            }
        }
        self.statement_count = 0;
    }
}
