//! Detect empty loops statements.
use crate::lint::{
    utils::{add_diagnostic_and_emit, LintConfig},
    visitor::ExpressionAnalysisVisitor,
};
use codespan::FileId;
use codespan_reporting::diagnostic::Diagnostic;
use move_model::{
    ast::{Exp, ExpData},
    model::{FunctionEnv, GlobalEnv, NodeId},
};
#[derive(Debug)]
pub struct EmptyLoopVisitor;

impl Default for EmptyLoopVisitor {
    fn default() -> Self {
        Self::new()
    }
}

impl EmptyLoopVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn check_for_empty_loop(
        &self,
        node_id: &NodeId,
        loop_body: &Exp,
        env: &GlobalEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        match loop_body.as_ref() {
            ExpData::Call(_, _, args) => {
                if args.is_empty() {
                    let message = "Loop has no code. Did you forget to implement?";
                    add_diagnostic_and_emit(
                        &env.get_node_loc(*node_id),
                        message,
                        codespan_reporting::diagnostic::Severity::Warning,
                        env,
                        diags,
                    );
                }
            },
            ExpData::IfElse(_, _, body, then) => {
                self.check_for_empty_loop(node_id, body, env, diags);
                self.check_for_empty_loop(node_id, then, env, diags);
            },
            _ => (),
        }
    }
}

impl ExpressionAnalysisVisitor for EmptyLoopVisitor {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        _: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        if let ExpData::Loop(node_id, loop_body) = exp {
            self.check_for_empty_loop(node_id, loop_body, env, diags);
        }
    }
}
