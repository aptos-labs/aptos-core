// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{LoadedFunction, RuntimeEnvironment};
use move_vm_types::{instr::Instruction, values::Locals};
use std::collections::BTreeSet;

pub mod interpreter;
pub mod stdin;

use crate::debug::interpreter::InterpreterDebugInterface;

enum DebugCommandMode {
    Stdin,
}

pub struct DebugContext {
    command_mode: DebugCommandMode,
    current_op: DebuggerOp,
    breakpoints: BTreeSet<String>,
}

#[derive(Debug)]
#[allow(unused)]
enum DebuggerOp {
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
    pub(crate) fn debug_loop(
        &mut self,
        function: &LoadedFunction,
        locals: &Locals,
        pc: u16,
        instr: &Instruction,
        runtime_environment: &RuntimeEnvironment,
        interpreter: &dyn InterpreterDebugInterface,
    ) {
        let instr_string = format!("{:?}", instr);
        let function_string = function.name_as_pretty_string();

        let function_string_with_pc = format!("{}::{}", function_string, pc);
        let breakpoint_hit = self.breakpoints.iter().any(|bp| {
            instr_string[..].starts_with(bp.as_str()) || function_string_with_pc.contains(bp)
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
        }
    }
}
