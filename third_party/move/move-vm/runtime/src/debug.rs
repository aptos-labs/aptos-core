// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    interpreter::InterpreterDebugInterface, source_locator, LoadedFunction, RuntimeEnvironment,
};
use move_vm_types::{instr::Instruction, values::Locals};
use std::{
    collections::{BTreeSet, VecDeque},
    env,
    io::{self, IsTerminal, Write},
    str::FromStr,
};

// ── Batch-command queue ───────────────────

thread_local! {
    /// Pre-loaded command queue populated from [`MOVE_VM_STEP_COMMANDS_ENV_VAR_NAME`].
    /// Commands are comma-separated, e.g. `step,stack,step,continue`.
    ///
    /// When a command is available it is consumed without reading stdin,
    /// making the debugger scriptable for non-interactive use.
    static COMMAND_QUEUE: RefCell<VecDeque<String>> = {
        let queue = env::var(crate::tracing::MOVE_VM_STEP_COMMANDS_ENV_VAR_NAME)
            .map(|s| s.split(',').map(|c| c.trim().to_owned()).collect())
            .unwrap_or_default();
        RefCell::new(queue)
    };
}

use std::cell::RefCell;

#[derive(Debug)]
#[allow(unused)]
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
#[allow(unused)]
pub(crate) struct DebugContext {
    breakpoints: BTreeSet<String>,
    input_checker: InputChecker,
}

#[derive(Debug)]
#[allow(unused)]
enum InputChecker {
    StepRemaining(usize),
    StepOverRemaining {
        stack_depth: usize,
        remaining: usize,
    },
    StepOut {
        target_stack_depth: usize,
    },
    Continue,
}

impl DebugContext {
    #[allow(unused)]
    pub(crate) fn new() -> Self {
        Self {
            breakpoints: BTreeSet::new(),
            input_checker: InputChecker::StepRemaining(1),
        }
    }

    #[allow(unused)]
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
        let function_string = format!("{}::{}", function.name_as_pretty_string(), pc);
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
                stack_depth,
                remaining,
            } => {
                if *stack_depth >= interpreter.get_stack_frames(usize::MAX).stack_trace().len() {
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

            // Print function + source location.
            print!("function >> {}", function_string);
            if let Some(module_id) = function.module_id() {
                if let Some(loc) =
                    source_locator::get_bytecode_source_location(module_id, function.index(), pc)
                {
                    print!("  (at {})", loc);
                }
            }
            println!();
            println!("instruction >> {:?}\nprogram counter >> {}", instr, pc);

            loop {
                // ── Try the pre-loaded command queue first ────────────────────
                let queued = COMMAND_QUEUE.with(|q| q.borrow_mut().pop_front());
                let input_str = if let Some(cmd) = queued {
                    println!("> {}", cmd); // echo so output is parseable
                    cmd
                } else {
                    // ── Fall back to stdin; auto-continue when not a TTY ──────
                    if !io::stdin().is_terminal() {
                        self.input_checker = InputChecker::Continue;
                        break;
                    }
                    print!("> ");
                    std::io::stdout().flush().unwrap();
                    let mut line = String::new();
                    match io::stdin().read_line(&mut line) {
                        Ok(_) => line,
                        Err(err) => {
                            println!("Error reading input: {}", err);
                            break;
                        },
                    }
                };

                match input_str.parse::<DebugCommand>() {
                    Err(err) => println!("{}", err),
                    Ok(command) => match command {
                        DebugCommand::Step(n) => {
                            self.input_checker = InputChecker::StepRemaining(n);
                            break;
                        },
                        DebugCommand::StepOver(n) => {
                            self.input_checker = InputChecker::StepOverRemaining {
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
                            if function.local_tys().is_empty() {
                                println!("        Locals:");
                                println!("            (none)");
                            } else {
                                let mut s = String::new();
                                source_locator::print_locals_enriched(
                                    &mut s,
                                    function,
                                    locals,
                                    runtime_environment,
                                    false,
                                )
                                .unwrap();
                                println!("{}", s);
                            }
                        },
                    },
                }
            }
        }
    }
}
