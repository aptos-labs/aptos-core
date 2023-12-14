use crate::lint::visitor::{ExpDataVisitor, LintUtilities};
/// Detect if any code is too deeply nested (> 5 levels). This usually means the code can be buggy
/// and hard to read.
use move_model::{
    ast::ExpData,
    model::{FunctionEnv, GlobalEnv},
};

#[derive(Debug)]
pub struct DeepNestingVisitor {
    nesting_level: usize,
    max_nesting_allowed: usize,
}

impl Default for DeepNestingVisitor {
    fn default() -> Self {
        Self::new()
    }
}
impl DeepNestingVisitor {
    pub fn new() -> Self {
        Self {
            nesting_level: 0,
            max_nesting_allowed: 5,
        }
    }

    pub fn visitor() -> Box<dyn ExpDataVisitor> {
        Box::new(Self::new())
    }
}

impl ExpDataVisitor for DeepNestingVisitor {
    fn visit(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        if let Some(func) = func_env.get_def().as_ref() {
            func.visit_pre_post(
                &mut (|up: bool, exp: &ExpData| {

                    match exp {
                        ExpData::IfElse(node_id, _, _, _) |  ExpData::Loop(node_id, _) => {
                            if !up {
                                self.nesting_level += 1;
                            } else {
                                if self.nesting_level > 0 {
                                    self.nesting_level -= 1;
                                }
                                if self.nesting_level >= self.max_nesting_allowed {
                                    let message = format!(
                                        "Block nesting level exceeds allowed limit of {}. Consider refactoring your code.",
                                        self.max_nesting_allowed
                                    );
                                    self.add_diagnostic_and_emit(
                                        &env.get_node_loc(*node_id),
                                        &message,
                                        codespan_reporting::diagnostic::Severity::Warning,
                                        env
                                    );
                                }
                            }
                        }
                        _ => {}
                    }
                })
            );
        };
    }
}
impl LintUtilities for DeepNestingVisitor {}
