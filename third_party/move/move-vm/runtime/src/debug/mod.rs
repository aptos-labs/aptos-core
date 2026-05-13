// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{source_locator, LoadedFunction, RuntimeEnvironment};
use move_vm_types::{
    instr::Instruction,
    values::{debug::DebugValue, Locals},
};
use std::collections::{BTreeSet, HashMap};

pub mod dap;
pub mod interpreter;
pub mod stdin;

use crate::debug::interpreter::InterpreterDebugInterface;

enum DebugCommandMode {
    Stdin,
    DAP {
        event_tx: crossbeam_channel::Sender<dap::DapEvent>,
        command_rx: crossbeam_channel::Receiver<dap::DapCommand>,
    },
}

pub struct DebugContext {
    command_mode: DebugCommandMode,
    current_op: DebuggerOp,
    breakpoints: BTreeSet<String>,
    /// Snapshots of locals captured just before `MoveLoc` invalidates them.
    /// Key: call-stack depth at snapshot time → (local_index → last valid value).
    moved_locals: HashMap<usize, HashMap<usize, DebugValue>>,
    /// (source_loc, stack_depth) where we last stopped for a breakpoint.
    /// Suppresses duplicate hits on the same line at the same depth within
    /// one continue run, while still allowing hits in recursive/inner calls.
    last_breakpoint_sloc: Option<(String, usize)>,
}

#[derive(Debug)]
#[allow(unused)]
enum DebuggerOp {
    /// keep running until we're back at the same or shallower stack depth AND on a different source line
    StepOverLine {
        stack_depth: usize,
        start_source_loc: Option<String>,
    },
    /// pause after n more bytecode instructions
    StepRemaining(usize),
    /// like `StepRemaining` but only counts instructions at the same or shallower depth
    StepOverRemaining {
        stack_depth: usize,
        remaining: usize,
    },
    /// keep running until the stack shrinks to the target depth
    StepOut { target_stack_depth: usize },
    /// don't pause unless a breakpoint is hit
    Continue,
}

impl DebugContext {
    pub fn dap_handle(&self) -> Option<dap::DapDebugHandle> {
        match &self.command_mode {
            DebugCommandMode::DAP {
                event_tx,
                command_rx,
            } => Some(dap::DapDebugHandle {
                event_tx: event_tx.clone(),
                command_rx: command_rx.clone(),
            }),
            DebugCommandMode::Stdin => None,
        }
    }

    pub(crate) fn debug_loop(
        &mut self,
        function: &LoadedFunction,
        locals: &Locals,
        pc: u16,
        instr: &Instruction,
        runtime_environment: &RuntimeEnvironment,
        interpreter: &dyn InterpreterDebugInterface,
    ) {
        let current_stack_depth = interpreter.get_stack_depth();
        match instr {
            // record every MoveLoc to save locals for the later inspection in debugger
            Instruction::MoveLoc(idx) => {
                let idx = *idx as usize;
                if let Some(ty) = function.local_tys().get(idx) {
                    let resolver = source_locator::DebugTypeNameResolver::new(
                        runtime_environment,
                        interpreter,
                    );
                    let sv = move_vm_types::values::debug::serialize_value_for_debug(
                        locals, idx, ty, &resolver,
                    );
                    self.moved_locals
                        .entry(current_stack_depth)
                        .or_default()
                        .insert(idx, sv);
                }
            },
            Instruction::Ret => {
                self.moved_locals.remove(&current_stack_depth);
            },
            _ => {},
        }

        let instr_string = format!("{:?}", instr);
        let function_string = function.name_as_pretty_string();
        let current_sloc = function.module_id().and_then(|mid| {
            source_locator::get_bytecode_source_location(mid, function.index(), pc)
        });

        // clear last breakpoint if current `(sloc, depth)` is outside of the `self.last_breakpoint_sloc` location
        if let Some((ref prev_bp_sloc, prev_bp_depth)) = self.last_breakpoint_sloc {
            if current_stack_depth <= prev_bp_depth
                && current_sloc.as_deref() != Some(prev_bp_sloc.as_str())
            {
                self.last_breakpoint_sloc = None;
            }
        }
        let is_under_the_same_bp_sloc = match (&current_sloc, &self.last_breakpoint_sloc) {
            (Some(loc), Some((prev_bp_sloc, prev_bp_depth))) => {
                loc == prev_bp_sloc && current_stack_depth == *prev_bp_depth
            },
            _ => false,
        };
        let breakpoint_hit = !is_under_the_same_bp_sloc
            && self.breakpoints.iter().any(|bp| {
                instr_string[..].starts_with(bp.as_str())
                    || current_sloc.as_deref() == Some(bp.as_str())
            });

        let should_take_input = match &mut self.current_op {
            DebuggerOp::StepRemaining(n) => {
                if *n == 1 {
                    self.current_op = DebuggerOp::Continue;
                    true
                } else {
                    *n -= 1;
                    false
                }
            },
            DebuggerOp::StepOverLine {
                stack_depth,
                start_source_loc,
            } => {
                let current_depth = interpreter.get_stack_depth();
                if *stack_depth >= current_depth {
                    let line_changed = match (&current_sloc, &*start_source_loc) {
                        (Some(cur), Some(start)) => cur != start,
                        (Some(_), None) => true,
                        _ => false,
                    };
                    if line_changed {
                        self.current_op = DebuggerOp::Continue;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
            DebuggerOp::StepOverRemaining {
                stack_depth,
                remaining,
            } => {
                if *stack_depth >= interpreter.get_stack_depth() {
                    if *remaining == 1 {
                        self.current_op = DebuggerOp::Continue;
                        true
                    } else {
                        *remaining -= 1;
                        false
                    }
                } else {
                    false
                }
            },
            DebuggerOp::StepOut { target_stack_depth } => {
                if *target_stack_depth == interpreter.get_stack_depth() {
                    self.current_op = DebuggerOp::Continue;
                    true
                } else {
                    false
                }
            },
            DebuggerOp::Continue => false,
        };

        if !should_take_input && !breakpoint_hit {
            return;
        }

        let stop_reason = if breakpoint_hit {
            self.last_breakpoint_sloc = current_sloc
                .as_ref()
                .map(|loc| (loc.clone(), current_stack_depth));
            let bp_match = self
                .breakpoints
                .iter()
                .find(|bp| {
                    instr_string.starts_with(bp.as_str())
                        || current_sloc.as_deref() == Some(bp.as_str())
                })
                .cloned()
                .unwrap_or(function_string.clone());
            dap::StopReason::Breakpoint(bp_match)
        } else {
            dap::StopReason::Step
        };

        match &self.command_mode {
            DebugCommandMode::Stdin => self.apply_stdin_command_queue(
                function,
                locals,
                pc,
                instr,
                runtime_environment,
                interpreter,
                &instr_string,
                &function_string,
                breakpoint_hit,
            ),
            DebugCommandMode::DAP { .. } => self.apply_dap_command_queue(
                function,
                locals,
                pc,
                instr,
                runtime_environment,
                interpreter,
                &instr_string,
                &function_string,
                stop_reason,
                &current_sloc,
            ),
        }
    }
}
