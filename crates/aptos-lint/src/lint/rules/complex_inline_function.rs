/// Detect inline functions that have non-trivial code body (> 10 lines) and are used multiple times
/// This usually bloats the bytecode for a module and leads to much higher gas necessary.
/// Developers should consider making the function non-inline or refactor the code to reduce the
/// complexity.
use crate::lint::visitor::{ExpDataVisitor, LintUtilities};
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

    pub fn visitor() -> Box<dyn ExpDataVisitor> {
        Box::new(Self::new())
    }

    fn is_complex_and_frequently_used(&mut self, func_env: &FunctionEnv) -> bool {
        let is_inline = func_env.is_inline();

        let usage_frequency = self.get_usage_frequency(func_env);

        let not_defined_at_0x1 = !self.is_defined_at_0x1(func_env);

        is_inline && not_defined_at_0x1 && self.statement_count >= 10 && usage_frequency > 2
    }

    fn is_defined_at_0x1(&self, func_env: &FunctionEnv) -> bool {
        func_env.get_full_name_with_address().contains("0x1")
    }

    fn count_statements(&mut self, exp: &ExpData) {
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

impl ExpDataVisitor for ComplexInlineFunctionVisitor {
    fn visit(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        if let Some(func) = func_env.get_def().as_ref() {
            func.visit_pre_post(
                &mut (|up: bool, exp: &ExpData| {
                    if !up {
                        self.count_statements(exp);
                    }
                }),
            );

            let is_complex_and_frequently_used = self.is_complex_and_frequently_used(func_env);

            if is_complex_and_frequently_used {
                let message = "Complex inline function detected. Consider removing the inline modifier or refactoring to reduce complexity or usage.";
                self.add_diagnostic_and_emit(
                    &func_env.get_loc(),
                    message,
                    codespan_reporting::diagnostic::Severity::Warning,
                    env,
                );
            }
        }
        self.statement_count = 0;
    }
}

impl LintUtilities for ComplexInlineFunctionVisitor {}
