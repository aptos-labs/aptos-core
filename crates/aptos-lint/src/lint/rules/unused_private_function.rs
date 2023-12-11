// The visitor identifies functions that are declared but not used
// (i.e., functions that are defined but not called anywhere within the module).

use std::collections::BTreeMap;
use move_model::model::{ GlobalEnv, Visibility, ModuleEnv, Loc };
use crate::lint::visitor::{ ExpDataVisitor, LintUtilities };

pub struct UnusedFunctionVisitor {
    not_called_functions: BTreeMap<String, Loc>,
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
            if func_env.visibility() == Visibility::Private {
                if func_env.get_calling_functions().unwrap().len() == 0 {
                    let message = format!(
                        "Function '{}' is unused.",
                        env.symbol_pool().string(func_env.get_name()).to_string()
                    );
                    self.add_diagnostic_and_emit(
                        &func_env.get_loc(),
                        &message,
                        codespan_reporting::diagnostic::Severity::Warning,
                        env
                    );
                }
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
            self.add_diagnostic_and_emit(&loc, &message, codespan_reporting::diagnostic::Severity::Warning, env);
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
