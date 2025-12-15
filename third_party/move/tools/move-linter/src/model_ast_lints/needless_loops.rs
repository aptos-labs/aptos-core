// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Lints loops that always exit on their first iteration.

use move_compiler_v2::external_checks::ExpChecker;
use move_model::{
    ast::{Exp, ExpData, Operation},
    model::{FunId, FunctionEnv, NodeId},
};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct NeedlessLoops {
    current_fun: Option<FunId>,
    loop_to_cont: BTreeMap<NodeId, BTreeMap<NodeId, bool>>,
}

impl ExpChecker for NeedlessLoops {
    fn get_name(&self) -> String {
        "needless_loops".to_string()
    }

    fn visit_expr_pre(&mut self, fenv: &FunctionEnv, expr: &ExpData) {
        self.refresh_loop_bindings(fenv);

        let ExpData::Loop(loop_id, body) = expr else {
            return;
        };

        let Some(exit) = self.analyze_loop(fenv, *loop_id, body) else {
            return;
        };

        let env = fenv.env();
        let loc = env.get_node_loc(exit.node_id);
        if loc.is_inlined() {
            return;
        }
        self.report(
            env,
            &loc,
            &format!(
                "Needless loop: this loop always {} on first iteration, consider removing the loop",
                exit.kind.as_str()
            ),
        );
    }
}

impl NeedlessLoops {
    fn refresh_loop_bindings(&mut self, fenv: &FunctionEnv) {
        let fun_id = fenv.get_id();
        if self.current_fun == Some(fun_id) {
            return;
        }
        self.current_fun = Some(fun_id);
        self.loop_to_cont.clear();
        if let Some(def) = fenv.get_def() {
            let (loop_to_cont, _) = def.compute_loop_bindings();
            self.loop_to_cont = loop_to_cont;
        }
    }

    fn analyze_loop(&self, _fenv: &FunctionEnv, loop_id: NodeId, body: &Exp) -> Option<ExitInfo> {
        if self.loop_has_continue(loop_id) {
            return None;
        }

        let exit = self.first_exit(body.as_ref(), /*loop_nest*/ 0)?;
        // Disallow loops whose first meaningful statement is a continue to itself.
        if matches!(exit.kind, ExitKind::ContinueToSelf) {
            return None;
        }

        Some(exit.normalize())
    }

    fn loop_has_continue(&self, loop_id: NodeId) -> bool {
        self.loop_to_cont
            .get(&loop_id)
            .map(|conts| conts.values().any(|is_continue| *is_continue))
            .unwrap_or(false)
    }

    /// Finds the first meaningful instruction in `expr` and returns how it exits the current loop.
    fn first_exit(&self, expr: &ExpData, loop_nest: usize) -> Option<ExitInfo> {
        use ExpData::*;

        match expr {
            Sequence(_, exps) => self.first_exit_sequence(exps, loop_nest),
            Block(_, _, _, scope) => self.first_exit(scope.as_ref(), loop_nest),
            IfElse(id, _, if_true, if_false) => {
                let first = self.first_exit(if_true.as_ref(), loop_nest);
                let second = self.first_exit(if_false.as_ref(), loop_nest);
                match (first, second) {
                    (Some(a), Some(b)) => {
                        let kind = if a.kind == b.kind {
                            a.kind
                        } else {
                            ExitKind::Exits
                        };
                        Some(ExitInfo { node_id: *id, kind })
                    },
                    _ => None,
                }
            },
            Return(id, _) => Some(ExitInfo {
                node_id: *id,
                kind: ExitKind::Returns,
            }),
            Call(id, Operation::Abort, _) => Some(ExitInfo {
                node_id: *id,
                kind: ExitKind::Aborts,
            }),
            LoopCont(id, nest, is_continue) => {
                if *nest == loop_nest {
                    let kind = if *is_continue {
                        ExitKind::ContinueToSelf
                    } else {
                        ExitKind::Breaks
                    };
                    Some(ExitInfo { node_id: *id, kind })
                } else {
                    Some(ExitInfo {
                        node_id: *id,
                        kind: ExitKind::Exits,
                    })
                }
            },
            _ => None,
        }
    }

    fn first_exit_sequence(&self, exps: &[Exp], loop_nest: usize) -> Option<ExitInfo> {
        let mut iter = exps
            .iter()
            .enumerate()
            .filter(|(_, e)| !self.is_trivial(e.as_ref()));
        let (idx, head) = iter.next()?;

        if let Some(exit) = self.first_exit(head.as_ref(), loop_nest) {
            return Some(exit);
        }

        // If the head is an if/else, a branch may fall through to the rest of the sequence.
        if let ExpData::IfElse(id, _, if_true, if_false) = head.as_ref() {
            let tail = &exps[idx + 1..];
            let true_exit = self.first_exit(if_true.as_ref(), loop_nest);
            let false_exit = self.first_exit(if_false.as_ref(), loop_nest);
            let tail_exit = self.first_exit_sequence(tail, loop_nest);

            match (true_exit, false_exit, tail_exit) {
                (Some(branch), None, Some(rest)) | (None, Some(branch), Some(rest)) => {
                    let kind = if branch.kind == rest.kind {
                        branch.kind
                    } else {
                        ExitKind::Exits
                    };
                    return Some(ExitInfo { node_id: *id, kind });
                },
                (None, None, rest) => return rest,
                _ => {},
            }
        }

        None
    }

    fn is_trivial(&self, expr: &ExpData) -> bool {
        matches!(
            expr,
            ExpData::Assign(..) | ExpData::Mutate(..) | ExpData::SpecBlock(..)
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExitKind {
    Returns,
    Aborts,
    Breaks,
    Exits,
    ContinueToSelf,
}

impl ExitKind {
    fn as_str(self) -> &'static str {
        match self {
            ExitKind::Returns => "returns",
            ExitKind::Aborts => "aborts",
            ExitKind::Breaks => "breaks",
            ExitKind::Exits | ExitKind::ContinueToSelf => "exits",
        }
    }
}

#[derive(Clone, Debug)]
struct ExitInfo {
    node_id: NodeId,
    kind: ExitKind,
}

impl ExitInfo {
    fn normalize(self) -> Self {
        let kind = match self.kind {
            ExitKind::ContinueToSelf => ExitKind::Exits,
            other => other,
        };
        Self { kind, ..self }
    }
}
