//! Detect loops with conditions that always trigger an exit. This means the loop can never run for
//! more than one iteration and is a sign it might have been incorrectly written.
use crate::lint::{
    utils::{add_diagnostic_and_emit, LintConfig},
    visitor::ExpressionAnalysisVisitor,
};
use codespan::FileId;

use codespan_reporting::diagnostic::Diagnostic;
use move_model::{
    ast::ExpData,
    model::{FunctionEnv, GlobalEnv, NodeId},
};
#[derive(Debug)]
pub struct UnconditionalExitLoopVisitor;

impl Default for UnconditionalExitLoopVisitor {
    fn default() -> Self {
        Self::new()
    }
}

fn does_exp_in_loop_always_exit(exp: &ExpData) -> bool {
    match exp {
        ExpData::Return(_, _) | ExpData::LoopCont(_, false) => true,

        ExpData::IfElse(_, _, then_exp, else_exp) => {
            does_exp_in_loop_always_exit(then_exp) && does_exp_in_loop_always_exit(else_exp)
        },

        ExpData::Block(_, _, _, body) => does_exp_in_loop_always_exit(body),

        ExpData::Sequence(_, exps) => exps
            .first()
            .map_or(false, |first_exp| does_exp_in_loop_always_exit(first_exp)),
        _ => false,
    }
}

impl UnconditionalExitLoopVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpressionAnalysisVisitor> {
        Box::new(Self::new())
    }

    fn check_for_unconditional_exit(
        &self,
        node_id: &NodeId,
        loop_body: &ExpData,
        env: &GlobalEnv,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        let always_exits = does_exp_in_loop_always_exit(loop_body);
        if always_exits {
            let message = "Loop always exits unconditionally. Consider revising the loop's logic.";
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

impl ExpressionAnalysisVisitor for UnconditionalExitLoopVisitor {
    fn post_visit_expression(
        &mut self,
        exp: &ExpData,
        _func_env: &FunctionEnv,
        env: &GlobalEnv,
        _: &LintConfig,
        diags: &mut Vec<Diagnostic<FileId>>,
    ) {
        if let ExpData::Loop(node_id, loop_body) = exp {
            self.check_for_unconditional_exit(node_id, loop_body.as_ref(), env, diags);
        }
    }
}
