//! Check for unsorted imports.
use crate::lint::{utils::add_diagnostic_and_emit_by_span, visitor::ExpressionAnalysisVisitor};
use move_model::model::{GlobalEnv, ModuleEnv};

#[derive(Debug)]
pub struct SortedImportsLint;

impl SortedImportsLint {
    fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn check_imports_sorted(&self, module_env: &ModuleEnv) {
        let imports = module_env.get_use_decls().to_vec();
        let imports_string = imports
            .iter()
            .map(|import| import.module_name.display_full(module_env.env).to_string())
            .collect::<Vec<_>>();
        let mut sorted_imports = imports.clone();
        sorted_imports.sort_by_key(|import| {
            import
                .module_name
                .name()
                .display(module_env.symbol_pool())
                .to_string()
        });
        let sorted_imports_string = sorted_imports
            .iter()
            .map(|import| import.module_name.display_full(module_env.env).to_string())
            .collect::<Vec<_>>();

        if imports_string != sorted_imports_string {
            let message = format!(
                "Imports in module {:?} are not sorted alphabetically.",
                module_env.get_name().display(module_env.env).to_string()
            );
            add_diagnostic_and_emit_by_span(
                imports.first().unwrap().loc.span(),
                imports.last().unwrap().loc.span(),
                imports.first().unwrap().loc.file_id(),
                &message,
                codespan_reporting::diagnostic::Severity::Warning,
                module_env.env,
            );
        }
    }
}

impl ExpressionAnalysisVisitor for SortedImportsLint {
    fn visit_module(&mut self, module_env: &ModuleEnv, _env: &GlobalEnv) {
        self.check_imports_sorted(module_env);
    }
}
