// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module implements a conservative stackless-bytecode linter that detects
//! needless loops: loops that always return, abort, or break on their first iteration.
//! The implementation intentionally avoids complex control-flow reasoning to keep
//! false positives at zero. It only reports when the first instruction in the loop
//! body is a `return`/`abort`/`break`, or when the first instruction is a `branch` whose
//! both targets immediately `return`, `abort`, or `break`.

use move_compiler_v2::external_checks::StacklessBytecodeChecker;
use move_stackless_bytecode::{
    function_target::FunctionTarget,
    stackless_bytecode::{AttrId, Bytecode, Label},
};
use std::collections::{HashMap, HashSet};

/// Linter for detecting needless loops.
pub struct NeedlessLoops {}

impl StacklessBytecodeChecker for NeedlessLoops {
    fn get_name(&self) -> String {
        "needless_loops".to_string()
    }

    fn check(&self, target: &FunctionTarget) {
        let analyzer = NeedlessLoopAnalyzer::new(target);
        let needless_loops = analyzer.find_needless_loops();

        for loop_info in needless_loops {
            let loc = target.get_bytecode_loc(loop_info.attr_id);
            if loc.is_inlined() {
                continue;
            }
            self.report(
                target.global_env(),
                &loc,
                &format!("Needless loop: this loop always {} on first iteration, consider removing the loop", loop_info.reason),
            );
        }
    }
}

#[derive(Debug)]
struct LoopInfo {
    attr_id: AttrId,
    reason: String,
}

struct NeedlessLoopAnalyzer<'a> {
    code: &'a [Bytecode],
    label_positions: HashMap<Label, usize>,
    loop_headers: HashSet<Label>,
}

impl<'a> NeedlessLoopAnalyzer<'a> {
    pub fn new(target: &'a FunctionTarget<'a>) -> Self {
        let code = target.get_bytecode();
        let mut label_positions = HashMap::new();
        let mut loop_headers = HashSet::new();

        for (pc, instr) in code.iter().enumerate() {
            if let Bytecode::Label(_, label) = instr {
                label_positions.insert(*label, pc);
            }
        }

        for (pc, instr) in code.iter().enumerate() {
            match instr {
                Bytecode::Jump(_, target_label) => {
                    if let Some(&target_pc) = label_positions.get(target_label) {
                        if target_pc <= pc {
                            loop_headers.insert(*target_label);
                        }
                    }
                },
                Bytecode::Branch(_, first_label, second_label, _) => {
                    if let Some(&first_pc) = label_positions.get(first_label) {
                        if first_pc <= pc {
                            loop_headers.insert(*first_label);
                        }
                    }
                    if let Some(&second_pc) = label_positions.get(second_label) {
                        if second_pc <= pc {
                            loop_headers.insert(*second_label);
                        }
                    }
                },
                _ => {},
            }
        }

        Self {
            code,
            label_positions,
            loop_headers,
        }
    }

    pub fn find_needless_loops(&self) -> Vec<LoopInfo> {
        let mut needless_loops = Vec::new();

        for &loop_header in &self.loop_headers {
            if let Some(loop_info) = self.analyze_simple_loop(loop_header) {
                needless_loops.push(loop_info);
            }
        }

        needless_loops
    }

    fn analyze_simple_loop(&self, loop_header: Label) -> Option<LoopInfo> {
        let loop_start_pc = *self.label_positions.get(&loop_header)?;

        let mut pc = loop_start_pc + 1;

        pc = self.advance_to_meaningful(pc);
        while pc < self.code.len() {
            let instr = &self.code[pc];

            match instr {
                Bytecode::Label(_, _) => {},
                Bytecode::Ret(attr_id, _) => {
                    return Some(LoopInfo {
                        attr_id: *attr_id,
                        reason: "returns".to_string(),
                    });
                },
                Bytecode::Abort(attr_id, _) => {
                    return Some(LoopInfo {
                        attr_id: *attr_id,
                        reason: "aborts".to_string(),
                    });
                },
                Bytecode::Jump(attr_id, target_label) => {
                    if self.is_definite_loop_exit(loop_header, *target_label) {
                        return Some(LoopInfo {
                            attr_id: *attr_id,
                            reason: "breaks".to_string(),
                        });
                    }
                    break;
                },
                Bytecode::Branch(attr_id, first_label, second_label, _) => {
                    let first_exit = self.deep_exit_analysis(loop_header, *first_label);
                    let second_exit = self.deep_exit_analysis(loop_header, *second_label);

                    match (first_exit, second_exit) {
                        (Some(exit1), Some(exit2)) if exit1 == exit2 => {
                            return Some(LoopInfo {
                                attr_id: *attr_id,
                                reason: exit1.to_string(),
                            });
                        },
                        _ => break,
                    }
                },
                _ => {
                    break;
                },
            }
            pc += 1;
        }

        None
    }

    fn deep_exit_analysis(&self, loop_header: Label, start_label: Label) -> Option<ImmediateExit> {
        let mut visited_labels = HashSet::new();
        self.follow_path_to_exit(loop_header, start_label, &mut visited_labels)
    }

    fn follow_path_to_exit(
        &self,
        loop_header: Label,
        current_label: Label,
        visited: &mut HashSet<Label>,
    ) -> Option<ImmediateExit> {
        if !visited.insert(current_label) {
            return None;
        }

        let mut pc = *self.label_positions.get(&current_label)?;

        pc = self.advance_to_meaningful(pc);

        while pc < self.code.len() {
            match &self.code[pc] {
                Bytecode::Label(_, _) => {
                    return None;
                },
                Bytecode::Ret(_, _) => {
                    return Some(ImmediateExit::Returns);
                },
                Bytecode::Abort(_, _) => {
                    return Some(ImmediateExit::Aborts);
                },
                Bytecode::Jump(_, target_label) => {
                    if self.is_definite_loop_exit(loop_header, *target_label) {
                        return Some(ImmediateExit::Breaks);
                    }
                    return None;
                },
                Bytecode::Branch(_, first_label, second_label, _) => {
                    let first_exit = self.follow_path_to_exit(loop_header, *first_label, visited);
                    let second_exit = self.follow_path_to_exit(loop_header, *second_label, visited);

                    match (first_exit, second_exit) {
                        (Some(exit1), Some(exit2)) if exit1 == exit2 => {
                            return Some(exit1);
                        },
                        _ => return None,
                    }
                },
                _ => {
                    pc += 1;
                    continue;
                },
            }
        }

        None
    }

    fn advance_to_meaningful(&self, mut pc: usize) -> usize {
        while pc < self.code.len() {
            match &self.code[pc] {
                Bytecode::Label(..)
                | Bytecode::Nop(..)
                | Bytecode::Load(..)
                | Bytecode::Assign(..)
                | Bytecode::Prop(..)
                | Bytecode::SpecBlock(..) => pc += 1,
                _ => break,
            }
        }
        pc
    }

    fn is_definite_loop_exit(&self, loop_header: Label, target_label: Label) -> bool {
        let header_pc = match self.label_positions.get(&loop_header) {
            Some(&pc) => pc,
            None => return false,
        };
        let target_pc = match self.label_positions.get(&target_label) {
            Some(&pc) => pc,
            None => return false,
        };

        if target_pc <= header_pc {
            return false;
        }

        if self.loop_headers.contains(&target_label) {
            return false;
        }

        for (pc, instr) in self.code.iter().enumerate().skip(target_pc) {
            match instr {
                Bytecode::Jump(_, lbl) => {
                    if let Some(&dest_pc) = self.label_positions.get(lbl) {
                        if dest_pc <= pc {
                            return false;
                        }
                    }
                },
                Bytecode::Branch(_, l1, l2, _) => {
                    if let Some(&d1) = self.label_positions.get(l1) {
                        if d1 <= pc {
                            return false;
                        }
                    }
                    if let Some(&d2) = self.label_positions.get(l2) {
                        if d2 <= pc {
                            return false;
                        }
                    }
                },
                _ => {},
            }
        }

        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ImmediateExit {
    Returns,
    Aborts,
    Breaks,
}

impl std::fmt::Display for ImmediateExit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImmediateExit::Returns => write!(f, "returns"),
            ImmediateExit::Aborts => write!(f, "aborts"),
            ImmediateExit::Breaks => write!(f, "breaks"),
        }
    }
}
