//! The lint identifies and warns about explicit self-assignments
//! in Move programs, such as `x = x;`. These assignments are redundant and do not affect the
//! program's logic. By detecting these patterns, the lint encourages cleaner and more efficient
//! code by suggesting the removal of unnecessary self-assignments. This lint enhances code
//! quality by focusing on eliminating operations that have no effect, thereby improving
//! readability and maintainability.
use crate::lint::utils::{add_diagnostic_and_emit, LintConfig};
use crate::lint::visitor::ExpressionAnalysisVisitor;
use codespan::FileId;

use codespan_reporting::diagnostic::Diagnostic;
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
    fn check_explicit_self_assignment(
        &self,
        exp: &ExpData,
        env: &GlobalEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        if let ExpData::Assign(node_id, Pattern::Var(_, lhs), exp) = exp {
            if let ExpData::LocalVar(_, rhs) = exp.as_ref() {
                if lhs == rhs {
                    let message = "Explicit self-assignment detected. Consider removing it.";
                    add_diagnostic_and_emit(
                        &env.get_node_loc(*node_id),
                        message,
                        codespan_reporting::diagnostic::Severity::Warning,
                        env,
                        diags,
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
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        self.check_explicit_self_assignment(exp, env, diags);
    }
}
