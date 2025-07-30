// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{interpreter::InterpreterDebugInterface, LoadedFunction, RuntimeEnvironment};
use move_binary_format::file_format::Bytecode;
use move_vm_types::values::{self, Locals};
use std::{
    collections::BTreeSet,
    io::{self, Write},
    str::FromStr,
};

#[derive(Debug)]
enum DebugCommand {
    PrintStack,
    Step(usize),
    StepOver(usize),
    StepOut,
    Continue,
    Breakpoint(String),
    DeleteBreakpoint(String),
    PrintBreakpoints,
}

impl DebugCommand {
    pub fn debug_string(&self) -> &str {
        match self {
            Self::PrintStack => "stack",
            Self::Step(_) => "step",
            Self::StepOver(_) => "step_over",
            Self::StepOut => "step_out",
            Self::Continue => "continue",
            Self::Breakpoint(_) => "breakpoint ",
            Self::DeleteBreakpoint(_) => "delete ",
            Self::PrintBreakpoints => "breakpoints",
        }
    }

    pub fn commands() -> Vec<DebugCommand> {
        vec![
            Self::PrintStack,
            Self::Step(0),
            Self::StepOver(0),
            Self::StepOut,
            Self::Continue,
            Self::Breakpoint("".to_string()),
            Self::DeleteBreakpoint("".to_string()),
            Self::PrintBreakpoints,
        ]
    }
}

fn parse_number(s: &str) -> Result<usize, String> {
    if s.trim_start().is_empty() {
        return Ok(1);
    }
    let n = s.trim_start().parse::<usize>();
    if n.is_err() {
        return Err("Step count must be a number".to_string());
    }
    Ok(n.unwrap())
}

impl FromStr for DebugCommand {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use DebugCommand::*;
        let s = s.trim();
        if s.starts_with(PrintStack.debug_string()) {
            return Ok(PrintStack);
        }

        if s.starts_with(StepOut.debug_string()) {
            return Ok(StepOut);
        }

        if let Some(n) = s.strip_prefix(StepOver(0).debug_string()) {
            return Ok(StepOver(parse_number(n)?));
        }

        if let Some(n) = s.strip_prefix(Step(0).debug_string()) {
            return Ok(Step(parse_number(n)?));
        }

        if s.starts_with(Continue.debug_string()) {
            return Ok(Continue);
        }
        if let Some(breakpoint) = s.strip_prefix(Breakpoint("".to_owned()).debug_string()) {
            return Ok(Breakpoint(breakpoint.to_owned()));
        }
        if let Some(breakpoint) = s.strip_prefix(DeleteBreakpoint("".to_owned()).debug_string()) {
            return Ok(DeleteBreakpoint(breakpoint.to_owned()));
        }
        if s.starts_with(PrintBreakpoints.debug_string()) {
            return Ok(PrintBreakpoints);
        }
        Err(format!(
            "Unrecognized command: {}\nAvailable commands: {}",
            s,
            Self::commands()
                .iter()
                .map(|command| command.debug_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

#[derive(Debug)]
pub(crate) struct DebugContext {
    breakpoints: BTreeSet<String>,
    input_checker: InputChecker,
}

#[derive(Debug)]
enum InputChecker {
    StepRemaining(usize),
    StepOverRemaining {
        function_string: String,
        stack_depth: usize,
        remaining: usize,
    },
    StepOut {
        target_stack_depth: usize,
    },
    Continue,
}

impl DebugContext {
    pub(crate) fn new() -> Self {
        Self {
            breakpoints: BTreeSet::new(),
            input_checker: InputChecker::StepRemaining(1),
        }
    }

    pub(crate) fn debug_loop(
        &mut self,
        function: &LoadedFunction,
        locals: &Locals,
        pc: u16,
        instr: &Bytecode,
        runtime_environment: &RuntimeEnvironment,
        interpreter: &dyn InterpreterDebugInterface,
    ) {
        let instr_string = format!("{:?}", instr);
        let function_string = function.name_as_pretty_string();
        let breakpoint_hit = self
            .breakpoints
            .iter()
            .any(|bp| instr_string[..].starts_with(bp.as_str()) || function_string.contains(bp));

        let should_take_input = match &mut self.input_checker {
            InputChecker::StepRemaining(n) => {
                if *n == 1 {
                    self.input_checker = InputChecker::Continue;
                    true
                } else {
                    *n -= 1;
                    false
                }
            },
            InputChecker::StepOverRemaining {
                function_string: target_function_string,
                stack_depth,
                remaining,
            } => {
                if &function_string == target_function_string
                    && *stack_depth == interpreter.get_stack_frames(usize::MAX).stack_trace().len()
                {
                    if *remaining == 1 {
                        self.input_checker = InputChecker::Continue;
                        true
                    } else {
                        *remaining -= 1;
                        false
                    }
                } else {
                    false
                }
            },
            InputChecker::StepOut { target_stack_depth } => {
                if *target_stack_depth
                    == interpreter.get_stack_frames(usize::MAX).stack_trace().len()
                {
                    self.input_checker = InputChecker::Continue;
                    true
                } else {
                    false
                }
            },
            InputChecker::Continue => false,
        };

        if should_take_input || breakpoint_hit {
            if breakpoint_hit {
                let bp_match = self
                    .breakpoints
                    .iter()
                    .find(|bp| instr_string.starts_with(bp.as_str()))
                    .cloned()
                    .unwrap_or(function_string.clone());
                println!(
                    "Breakpoint {} hit with instruction {}",
                    bp_match, instr_string
                );
            }
            println!(
                "function >> {}\ninstruction >> {:?}\nprogram counter >> {}",
                function_string, instr, pc
            );
            loop {
                print!("> ");
                std::io::stdout().flush().unwrap();
                let mut input = String::new();
                match io::stdin().read_line(&mut input) {
                    Ok(_) => match input.parse::<DebugCommand>() {
                        Err(err) => println!("{}", err),
                        Ok(command) => match command {
                            DebugCommand::Step(n) => {
                                self.input_checker = InputChecker::StepRemaining(n);
                                break;
                            },
                            DebugCommand::StepOver(n) => {
                                self.input_checker = InputChecker::StepOverRemaining {
                                    function_string: function_string.clone(),
                                    stack_depth: interpreter
                                        .get_stack_frames(usize::MAX)
                                        .stack_trace()
                                        .len(),
                                    remaining: n,
                                };
                                break;
                            },
                            DebugCommand::Continue => {
                                self.input_checker = InputChecker::Continue;
                                break;
                            },
                            DebugCommand::Breakpoint(breakpoint) => {
                                self.breakpoints.insert(breakpoint.to_string());
                            },
                            DebugCommand::DeleteBreakpoint(breakpoint) => {
                                self.breakpoints.remove(&breakpoint);
                            },
                            DebugCommand::StepOut => {
                                let stack_depth =
                                    interpreter.get_stack_frames(usize::MAX).stack_trace().len();
                                if stack_depth == 0 {
                                    println!("No stack frames to step out of");
                                } else {
                                    self.input_checker = InputChecker::StepOut {
                                        target_stack_depth: stack_depth - 1,
                                    };
                                    break;
                                }
                            },
                            DebugCommand::PrintBreakpoints => self
                                .breakpoints
                                .iter()
                                .enumerate()
                                .for_each(|(i, bp)| println!("[{}] {}", i, bp)),
                            DebugCommand::PrintStack => {
                                let mut s = String::new();
                                interpreter
                                    .debug_print_stack_trace(&mut s, runtime_environment)
                                    .unwrap();
                                println!("{}", s);
                                println!("Current frame: {}\n", function_string);
                                let code = function.code();
                                println!("        Code:");
                                for (i, instr) in code.iter().enumerate() {
                                    if i as u16 == pc {
                                        println!("          > [{}] {:?}", pc, instr);
                                    } else {
                                        println!("            [{}] {:?}", i, instr);
                                    }
                                }
                                println!("        Locals:");
                                if !function.local_tys().is_empty() {
                                    let mut s = String::new();
                                    values::debug::print_locals(&mut s, locals).unwrap();
                                    println!("{}", s);
                                } else {
                                    println!("            (none)");
                                }
                            },
                        },
                    },
                    Err(err) => {
                        println!("Error reading input: {}", err);
                        break;
                    },
                }
            }
        }
    }
}
