// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{DebugCommandMode, DebuggerOp};
use crate::{
    interpreter::InterpreterDebugInterface, source_locator, LoadedFunction, RuntimeEnvironment,
};
use move_vm_types::{
    values,
    values::{debug::DebugValue, Locals},
};
use std::collections::HashMap;

// ── Cloneable handle for propagating DAP channels across threads (e.g. rayon) ──

#[derive(Clone)]
pub struct DapDebugHandle {
    pub event_tx: crossbeam_channel::Sender<DapEvent>,
    pub command_rx: crossbeam_channel::Receiver<DapCommand>,
}

// ── Channel protocol types for external debug controllers (e.g. DAP servers) ──

#[derive(Debug)]
pub enum DapCommand {
    Continue,
    Step(usize),
    StepOver(usize),
    StepOut,
    SetBreakpoints(Vec<String>),
}

#[derive(Debug)]
pub enum StopReason {
    Entry,
    Step,
    Breakpoint(String),
}

#[derive(Debug)]
pub struct DapFrameInfo {
    pub function_name: String,
    pub pc: u16,
    pub source_location: Option<String>,
    pub locals: Vec<DapLocalInfo>,
}

#[derive(Debug)]
pub struct DapLocalInfo {
    pub index: usize,
    pub name: String,
    pub type_name: String,
    pub value: DebugValue,
}

#[derive(Debug)]
pub struct VmStoppedState {
    pub function_name: String,
    pub pc: u16,
    pub instruction: String,
    pub dap_stack_trace: Vec<DapFrameInfo>,
    pub dap_locals: Vec<DapLocalInfo>,
    pub source_location: Option<String>,
}

#[derive(Debug)]
pub enum DapEvent {
    Stopped {
        reason: StopReason,
        vm_state: VmStoppedState,
    },
    Terminated {
        message: Option<String>,
    },
}

/// Create a channel-based debug context without installing it.
///
/// Returns `(command_sender, event_receiver, event_sender, debug_context)`.
/// The caller sends `DapCommand`s to control the VM and receives `DapEvent`s
/// when the VM stops. The extra `event_sender` lets the VM host thread send
/// `DapEvent::Terminated` after execution finishes.
///
/// The returned `DebugContext` must be installed on the VM thread via
/// [`install_dap_debug_context_on_thread`].
pub fn create_dap_debug_context() -> (
    crossbeam_channel::Sender<DapCommand>,
    crossbeam_channel::Receiver<DapEvent>,
    crossbeam_channel::Sender<DapEvent>,
    super::DebugContext,
) {
    let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
    let (event_tx, event_rx) = crossbeam_channel::unbounded();
    let event_tx_clone = event_tx.clone();
    let debug_ctx = super::DebugContext::with_channels(event_tx, cmd_rx);
    (cmd_tx, event_rx, event_tx_clone, debug_ctx)
}

/// Install a DAP debug context on the current thread and enable debugging.
/// Must be called on the VM execution thread.
pub fn install_dap_debug_context_on_thread(ctx: super::DebugContext) {
    crate::tracing::set_debugging_enabled(true);
    crate::tracing::set_debug_context(ctx);
}

// ── DebugContext DAP methods ────────────────────────────────────────────────

impl super::DebugContext {
    pub(crate) fn with_channels(
        event_tx: crossbeam_channel::Sender<DapEvent>,
        command_rx: crossbeam_channel::Receiver<DapCommand>,
    ) -> Self {
        Self {
            breakpoints: std::collections::BTreeSet::new(),
            current_op: DebuggerOp::StepRemaining(1),
            command_mode: DebugCommandMode::DAP {
                event_tx,
                command_rx,
            },
            moved_locals: HashMap::new(),
            last_breakpoint_sloc: None,
        }
    }

    pub(crate) fn apply_dap_command_queue(
        &mut self,
        function: &LoadedFunction,
        locals: &Locals,
        pc: u16,
        _instr: &move_vm_types::instr::Instruction,
        runtime_environment: &RuntimeEnvironment,
        interpreter: &dyn InterpreterDebugInterface,
        instr_string: &str,
        function_string: &str,
        stop_reason: StopReason,
        source_loc: &Option<String>,
    ) {
        let DebugCommandMode::DAP {
            event_tx,
            command_rx,
        } = &self.command_mode
        else {
            return;
        };

        let vm_stopped_state = build_vm_stopped_state(
            function,
            locals,
            pc,
            instr_string,
            function_string,
            runtime_environment,
            interpreter,
            &self.moved_locals,
        );

        if event_tx
            .send(DapEvent::Stopped {
                reason: stop_reason,
                vm_state: vm_stopped_state,
            })
            .is_err()
        {
            self.current_op = DebuggerOp::Continue;
            return;
        }

        // apply all pending commands from the command channel
        loop {
            let cmd = match command_rx.recv() {
                Ok(cmd) => cmd,
                Err(_) => {
                    self.current_op = DebuggerOp::Continue;
                    return;
                },
            };
            match cmd {
                DapCommand::Continue => {
                    self.current_op = DebuggerOp::Continue;
                    break;
                },
                DapCommand::Step(n) => {
                    self.current_op = DebuggerOp::StepRemaining(n);
                    break;
                },
                DapCommand::StepOver(_) => {
                    self.current_op = DebuggerOp::StepOverLine {
                        stack_depth: interpreter.get_stack_depth(),
                        start_source_loc: source_loc.clone(),
                    };
                    break;
                },
                DapCommand::StepOut => {
                    let stack_depth = interpreter.get_stack_depth();
                    if stack_depth == 0 {
                        self.current_op = DebuggerOp::Continue;
                    } else {
                        self.current_op = DebuggerOp::StepOut {
                            target_stack_depth: stack_depth - 1,
                        };
                    }
                    break;
                },
                DapCommand::SetBreakpoints(bps) => {
                    self.breakpoints = bps.into_iter().collect();
                },
            }
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn build_dap_local_infos(
    function: &LoadedFunction,
    locals: &Locals,
    runtime_environment: &RuntimeEnvironment,
    interpreter: &dyn InterpreterDebugInterface,
    moved_locals: Option<&HashMap<usize, DebugValue>>,
) -> Vec<DapLocalInfo> {
    if function.local_tys().is_empty() {
        return vec![];
    }
    let local_infos = source_locator::build_local_infos(function);
    let name_resolver =
        source_locator::DebugTypeNameResolver::new(runtime_environment, interpreter);

    local_infos
        .into_iter()
        .map(|local_info| {
            let ty = &function.local_tys()[local_info.index];
            let debug_value = values::debug::serialize_value_for_debug(
                locals,
                local_info.index,
                ty,
                &name_resolver,
            );
            // if it's Value::Invalid, it might be one of the `MoveLoc(idx)` invalidated locals
            let debug_value = if matches!(&debug_value, DebugValue::Invalid) {
                moved_locals
                    .and_then(|moved_locals| moved_locals.get(&local_info.index))
                    .cloned()
                    .unwrap_or(debug_value)
            } else {
                debug_value
            };
            DapLocalInfo {
                index: local_info.index,
                name: local_info.name,
                type_name: String::new(),
                value: debug_value,
            }
        })
        .collect()
}

fn build_vm_stopped_state(
    function: &LoadedFunction,
    locals: &Locals,
    pc: u16,
    instr_string: &str,
    function_string: &str,
    runtime_environment: &RuntimeEnvironment,
    interpreter: &dyn InterpreterDebugInterface,
    moved_locals: &HashMap<usize, HashMap<usize, DebugValue>>,
) -> VmStoppedState {
    let source_location = function.module_id().and_then(|module_id| {
        source_locator::get_bytecode_source_location(module_id, function.index(), pc)
    });

    let stack_depth = interpreter.get_stack_depth();
    let dap_stack_trace = interpreter
        .get_stack_frames(usize::MAX)
        .stack_trace()
        .iter()
        .enumerate()
        .map(|(i, (module_id, func_def_idx, code_offset))| {
            let frame_source_loc = module_id.as_ref().and_then(|mid| {
                source_locator::get_bytecode_source_location(mid, *func_def_idx, *code_offset)
            });
            let caller_depth = stack_depth - 1 - i;
            let moved_locals_at_frame = moved_locals.get(&caller_depth);
            let (frame_fname, frame_locals_infos) = match interpreter.get_frame_locals(i) {
                Some((func, locs)) => (
                    func.name_as_pretty_string(),
                    build_dap_local_infos(
                        func,
                        locs,
                        runtime_environment,
                        interpreter,
                        moved_locals_at_frame,
                    ),
                ),
                None => {
                    let name = module_id
                        .as_ref()
                        .map(|mid| format!("{}::{}", mid, func_def_idx))
                        .unwrap_or_else(|| format!("<script>::{}", func_def_idx));
                    (name, vec![])
                },
            };
            DapFrameInfo {
                function_name: frame_fname,
                pc: *code_offset,
                source_location: frame_source_loc,
                locals: frame_locals_infos,
            }
        })
        .collect();

    let current_moved_locals = moved_locals.get(&stack_depth);
    let local_infos = build_dap_local_infos(
        function,
        locals,
        runtime_environment,
        interpreter,
        current_moved_locals,
    );

    VmStoppedState {
        function_name: function_string.to_string(),
        pc,
        instruction: instr_string.to_string(),
        dap_stack_trace,
        dap_locals: local_infos,
        source_location,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_channel_protocol_round_trip() {
        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded();
        let (evt_tx, evt_rx) = crossbeam_channel::unbounded();

        let vm_thread = thread::spawn(move || {
            let evt = evt_rx.recv().unwrap();
            match &evt {
                DapEvent::Stopped {
                    reason,
                    vm_state: state,
                } => {
                    assert!(matches!(reason, StopReason::Step));
                    assert_eq!(state.function_name, "test_module::test_fn::0");
                    assert_eq!(state.pc, 0);
                },
                _ => panic!("expected Stopped event"),
            }
            cmd_tx.send(DapCommand::Step(1)).unwrap();

            let evt = evt_rx.recv().unwrap();
            assert!(matches!(evt, DapEvent::Stopped { .. }));
            cmd_tx.send(DapCommand::Continue).unwrap();

            let evt = evt_rx.recv().unwrap();
            assert!(matches!(evt, DapEvent::Terminated { .. }));
        });

        evt_tx
            .send(DapEvent::Stopped {
                reason: StopReason::Step,
                vm_state: VmStoppedState {
                    function_name: "test_module::test_fn::0".to_string(),
                    pc: 0,
                    instruction: "Call(0)".to_string(),
                    dap_stack_trace: vec![DapFrameInfo {
                        function_name: "test_module::test_fn".to_string(),
                        pc: 0,
                        source_location: Some("test.move:10".to_string()),
                        locals: vec![],
                    }],
                    dap_locals: vec![DapLocalInfo {
                        index: 0,
                        name: "x".to_string(),
                        type_name: "u64".to_string(),
                        value: DebugValue::Primitive("42".to_string()),
                    }],
                    source_location: Some("test.move:10".to_string()),
                },
            })
            .unwrap();

        let cmd = cmd_rx.recv().unwrap();
        assert!(matches!(cmd, DapCommand::Step(1)));

        evt_tx
            .send(DapEvent::Stopped {
                reason: StopReason::Breakpoint("test_module::test_fn".to_string()),
                vm_state: VmStoppedState {
                    function_name: "test_module::test_fn::1".to_string(),
                    pc: 1,
                    instruction: "Ret".to_string(),
                    dap_stack_trace: vec![],
                    dap_locals: vec![],
                    source_location: None,
                },
            })
            .unwrap();

        let cmd = cmd_rx.recv().unwrap();
        assert!(matches!(cmd, DapCommand::Continue));

        evt_tx.send(DapEvent::Terminated { message: None }).unwrap();

        vm_thread.join().unwrap();
    }

    #[test]
    fn test_create_and_install_dap_debug_context() {
        let (cmd_tx, evt_rx, _evt_tx, ctx) = create_dap_debug_context();
        install_dap_debug_context_on_thread(ctx);
        drop(cmd_tx);
        drop(evt_rx);
    }

    #[test]
    fn test_set_breakpoints_replaces_all() {
        let (cmd_tx, _evt_rx) = crossbeam_channel::unbounded();
        let (_evt_tx, cmd_rx) = crossbeam_channel::unbounded();
        let mut ctx = super::super::DebugContext::with_channels(cmd_tx, cmd_rx);

        ctx.breakpoints.insert("old_bp".to_string());
        assert!(ctx.breakpoints.contains("old_bp"));

        let new_bps = vec!["bp1".to_string(), "bp2".to_string()];
        ctx.breakpoints = new_bps.into_iter().collect();

        assert!(!ctx.breakpoints.contains("old_bp"));
        assert!(ctx.breakpoints.contains("bp1"));
        assert!(ctx.breakpoints.contains("bp2"));
    }
}
