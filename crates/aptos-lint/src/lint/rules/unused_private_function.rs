use crate::lint::visitor::{ExpDataVisitor, LintUtilities};
use move_model::model::{GlobalEnv, Loc, ModuleEnv, Visibility};
/// Detect private functions that are declared but not used
use std::collections::BTreeMap;

pub struct UnusedFunctionVisitor {
    not_called_functions: BTreeMap<String, Loc>,
}

impl Default for UnusedFunctionVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl UnusedFunctionVisitor {
    pub fn new() -> Self {
        Self {
            not_called_functions: BTreeMap::new(),
        }
    }

    pub fn visitor() -> Box<dyn ExpDataVisitor> {
        Box::new(Self::new())
    }

    fn detect_unused_functions(&mut self, module: &ModuleEnv, env: &GlobalEnv) {
        for func_env in module.get_functions() {
            if func_env.visibility() == Visibility::Private
                && func_env.get_calling_functions().unwrap().is_empty()
            {
                let message = format!(
                    "Function '{}' is unused.",
                    env.symbol_pool().string(func_env.get_name())
                );
                self.add_diagnostic_and_emit(
                    &func_env.get_loc(),
                    &message,
                    codespan_reporting::diagnostic::Severity::Warning,
                    env,
                );
            }
        }
    }

    fn check_unused_functions(&mut self, env: &GlobalEnv) {
        let mut warnings = Vec::new();
        for (fname, loc) in self.not_called_functions.iter() {
            let message = format!("Function '{}' is unused.", fname);
            warnings.push((loc.clone(), message));
        }
        for (loc, message) in warnings {
            self.add_diagnostic_and_emit(
                &loc,
                &message,
                codespan_reporting::diagnostic::Severity::Warning,
                env,
            );
        }
    }
}

impl ExpDataVisitor for UnusedFunctionVisitor {
    fn visit_module(&mut self, module: &ModuleEnv, env: &GlobalEnv) {
        self.detect_unused_functions(module, env);
        self.check_unused_functions(env);
    }
}

impl LintUtilities for UnusedFunctionVisitor {}
