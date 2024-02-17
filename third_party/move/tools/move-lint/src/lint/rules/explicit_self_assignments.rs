use crate::lint::utils::{add_diagnostic_and_emit, LintConfig};
use crate::lint::visitor::ExpressionAnalysisVisitor;
use move_model::ast::{ExpData, Pattern};
use move_model::model::{FunctionEnv, GlobalEnv};

pub struct ExplicitSelfAssignmentsVisitor;

impl Default for ExplicitSelfAssignmentsVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ExplicitSelfAssignmentsVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    /// Checks for explicit self-assignments in expressions.
    fn check_explicit_self_assignment(&self, exp: &ExpData, env: &GlobalEnv) {
        if let ExpData::Assign(node_id, Pattern::Var(_, lhs), exp) = exp {
            if let ExpData::LocalVar(_, rhs) = exp.as_ref() {
                if lhs == rhs {
                    let message = "Explicit self-assignment detected. Consider removing it.";
                    add_diagnostic_and_emit(
                        &env.get_node_loc(*node_id),
                        message,
                        codespan_reporting::diagnostic::Severity::Warning,
                        env,
                    );
                }
            }
        }
    }
}

impl ExpressionAnalysisVisitor for ExplicitSelfAssignmentsVisitor {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        _func_env: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
    ) {
        self.check_explicit_self_assignment(exp, env);
    }
}
