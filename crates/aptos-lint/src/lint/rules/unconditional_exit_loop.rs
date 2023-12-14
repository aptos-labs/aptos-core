use crate::lint::visitor::{ExpDataVisitor, LintUtilities};
/// Detect loops with conditions that always trigger an exit. This means the loop can never run for
/// more than one iteration and is a sign it might have been incorrectly written.
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

fn is_loop_always_exiting(exp: &ExpData) -> bool {
    match exp {
        ExpData::Return(_, _) | ExpData::LoopCont(_, false) => true,

        ExpData::IfElse(_, _, then_exp, else_exp) => {
            is_loop_always_exiting(then_exp) && is_loop_always_exiting(else_exp)
        },

        ExpData::Block(_, _, _, body) => is_loop_always_exiting(body),

        ExpData::Sequence(_, exps) => exps
            .first()
            .map_or(false, |last_exp| is_loop_always_exiting(last_exp)),
        _ => false,
    }
}

impl UnconditionalExitLoopVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpDataVisitor> {
        Box::new(Self::new())
    }

    fn check_for_unconditional_exit(&self, node_id: &NodeId, loop_body: &ExpData, env: &GlobalEnv) {
        let always_exits = is_loop_always_exiting(loop_body);
        if always_exits {
            let message = "Loop always exits unconditionally. Consider revising the loop's logic.";
            self.add_diagnostic_and_emit(
                &env.get_node_loc(*node_id),
                message,
                codespan_reporting::diagnostic::Severity::Warning,
                env,
            );
        }
    }
}

impl ExpDataVisitor for UnconditionalExitLoopVisitor {
    fn visit(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        if let Some(func) = func_env.get_def().as_ref() {
            func.visit_pre_post(
                &mut (|up: bool, exp: &ExpData| {
                    if let ExpData::Loop(node_id, loop_body) = exp {
                        if !up {
                            self.check_for_unconditional_exit(node_id, loop_body.as_ref(), env);
                        }
                    }
                }),
            );
        };
    }
}

impl LintUtilities for UnconditionalExitLoopVisitor {}
