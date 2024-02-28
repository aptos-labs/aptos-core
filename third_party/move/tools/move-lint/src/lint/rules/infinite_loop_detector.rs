//! `InfiniteLoopDetectorVisitor` identifies potential infinite loops in Move programs by checking for loops without 'break' or 'return'.
//! It warns about any `loop` constructs that may run indefinitely, promoting better control flow and program safety.
use crate::lint::utils::{add_diagnostic_and_emit, LintConfig};
use crate::lint::visitor::ExpressionAnalysisVisitor;
use codespan::FileId;

use codespan_reporting::diagnostic::Diagnostic;
use move_model::ast::{Exp, ExpData};
use move_model::model::{FunctionEnv, GlobalEnv};
pub struct InfiniteLoopDetectorVisitor;

impl Default for InfiniteLoopDetectorVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl InfiniteLoopDetectorVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    /// Checks for loop or while(true) without break or return.
    fn check_infinite_loop(
        &self,
        exp: &ExpData,
        env: &GlobalEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        if let ExpData::Loop(_, body) = exp {
            if !self.contains_break_or_return(body) {
                let message =
                    "Potential infinite loop detected. No 'break' or 'return' found in the loop.";
                add_diagnostic_and_emit(
                    &env.get_node_loc(exp.node_id()),
                    message,
                    codespan_reporting::diagnostic::Severity::Warning,
                    env,
                    diags,
                );
            }
        }
    }

    /// Recursively checks if the loop body contains a break or return statement.
    fn contains_break_or_return(&self, exp: &Exp) -> bool {
        let mut contains_break_or_return = false;
        exp.visit_pre_post(&mut |is_pre_visit, exp: &ExpData| {
            if !is_pre_visit {
                match exp {
                    ExpData::LoopCont(_, false) => contains_break_or_return = true,
                    ExpData::Return(_, _) => contains_break_or_return = true,
                    _ => {},
                }
            }
        });
        contains_break_or_return
    }
}

impl ExpressionAnalysisVisitor for InfiniteLoopDetectorVisitor {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        _func_env: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        self.check_infinite_loop(exp, env, diags);
    }
}
