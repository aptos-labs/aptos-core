use move_model::{ model::{ FunctionEnv, GlobalEnv, NodeId }, ast::{ ExpData, Exp } };

use crate::lint::visitor::{ ExpDataVisitor, LintUtilities };

#[derive(Debug)]
pub struct EmptyLoopVisitor;

impl EmptyLoopVisitor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn visitor() -> Box<dyn ExpDataVisitor> {
        Box::new(Self::new())
    }

    fn check_for_empty_loop(&self, node_id: &NodeId, loop_body: &Exp, env: &GlobalEnv) {
        match loop_body.as_ref() {
            ExpData::Call(_, _, exp_vec) => {
                if exp_vec.len() == 0 {
                    let message = "Empty loop detected. Consider removing or populating the loop.";
                    self.add_diagnostic_and_emit(
                        &env.get_node_loc(*node_id),
                        message,
                        codespan_reporting::diagnostic::Severity::Warning,
                        env
                    );
                }
            }
            ExpData::IfElse(_, _, e2, _) => {
                if let ExpData::Call(_, _, exp_vec) = e2.as_ref() {
                    if exp_vec.len() == 0 {
                        let message = "Empty ifelse detected. Consider removing or populating the loop.";
                        self.add_diagnostic_and_emit(
                            &env.get_node_loc(*node_id),
                            message,
                            codespan_reporting::diagnostic::Severity::Warning,
                            env
                        );
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
}

impl ExpDataVisitor for EmptyLoopVisitor {
    fn visit(&mut self, func_env: &FunctionEnv, env: &GlobalEnv) {
        func_env.get_def().map(|func| {
            func.visit_pre_post(
                &mut (|up: bool, exp: &ExpData| {
                    if let ExpData::Loop(node_id, loop_body) = exp {
                        if !up {
                            self.check_for_empty_loop(node_id, loop_body, env);
                        }
                    }
                })
            );
        });
    }
}
impl LintUtilities for EmptyLoopVisitor {}
