// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    proto,
    server::{
        replay::{ReplayTransactionSession, SCOPE_TRANSACTION_INFO},
        variables::{frame_locals_ref_id, StoredVariables},
    },
    utils::{parse_source_location, trim_hex_address},
};
use anyhow::Result;
use dap::{
    events::OutputEventBody,
    requests::{Command, Request},
    responses::ResponseBody,
    types::{
        Capabilities, OutputEventCategory, Scope, ScopePresentationhint, StackFrame,
        StoppedEventReason, Thread,
    },
};
use move_vm_runtime::debug::dap::{DapCommand, DapEvent, StopReason, VmStoppedState};
use std::{
    collections::BTreeMap,
    fmt::Display,
    io,
    io::{BufReader, BufWriter},
    path::PathBuf,
    thread,
};

// const SCOPE_TRANSACTION_INFO: i64 = 1;
// pub const SCOPE_LOCALS_BASE: i64 = 1000;

mod replay;
mod test;
pub mod variables;

#[derive(Debug, Clone)]
pub enum RunCommand {
    Test {
        filter: String,
        package_path: PathBuf,
        skip_fetch_latest_git_deps: bool,
    },
    Replay {
        txn_id: u64,
        network: String,
        local_packages: Vec<PathBuf>,
        prebuilt_packages: Vec<PathBuf>,
        named_addresses: BTreeMap<String, aptos_types::account_address::AccountAddress>,
        skip_fetch_latest_git_deps: bool,
    },
}

pub struct DapServer<R: io::Read, W: io::Write> {
    server: dap::server::Server<R, W>,
    mode: RunCommand,
    txn_session: Option<ReplayTransactionSession>,
    rt: tokio::runtime::Runtime,
    cmd_tx: Option<crossbeam_channel::Sender<DapCommand>>,
    event_rx: Option<crossbeam_channel::Receiver<DapEvent>>,
    vm_thread: Option<thread::JoinHandle<Result<()>>>,
    vm_stopped_state: Option<VmStoppedState>,
    pending_breakpoints: Vec<String>,
    stored_variables: StoredVariables,
}

impl<R: io::Read, W: io::Write> DapServer<R, W> {
    pub fn new(input: BufReader<R>, output: BufWriter<W>, mode: RunCommand) -> Result<Self> {
        Ok(Self {
            server: dap::server::Server::new(input, output),
            mode,
            txn_session: None,
            rt: tokio::runtime::Runtime::new()?,
            cmd_tx: None,
            event_rx: None,
            vm_thread: None,
            vm_stopped_state: None,
            pending_breakpoints: vec![],
            stored_variables: StoredVariables::new(),
        })
    }

    pub fn run(&mut self) -> Result<()> {
        loop {
            let Some(req) = self.server.poll_request()? else {
                break;
            };
            let result = match req.command.clone() {
                Command::Initialize(args) => self.handle_initialize(req, args),
                Command::Launch(args) => self.handle_launch(req, args),
                Command::SetBreakpoints(args) => self.handle_set_breakpoints(req, args),
                Command::ConfigurationDone => self.handle_configuration_done(req),
                Command::Threads => self.handle_threads(req),
                Command::Continue(args) => self.handle_continue(req, args),
                Command::Next(args) => self.handle_next(req, args),
                Command::StepIn(args) => self.handle_step_in(req, args),
                Command::StepOut(args) => self.handle_step_out(req, args),
                Command::StackTrace(args) => self.handle_stack_trace(req, args),
                Command::Scopes(args) => self.handle_scopes(req, args),
                Command::Variables(args) => self.handle_variables(req, args),
                Command::Disconnect(args) => self.handle_disconnect(req, args),
                _ => {
                    self.server.respond(req.error("unsupported command"))?;
                    Ok(())
                },
            };
            if let Err(e) = result {
                let _ = self.send_console(format_args!("error handling request: {e}"));
            }
        }
        Ok(())
    }

    fn handle_initialize(
        &mut self,
        req: Request,
        _args: dap::requests::InitializeArguments,
    ) -> Result<()> {
        let response = req.success(ResponseBody::Initialize(Capabilities {
            supports_configuration_done_request: Some(true),
            supports_single_thread_execution_requests: Some(true),
            ..Default::default()
        }));
        self.server.respond(response)?;
        self.server.send_event(dap::events::Event::Initialized)?;
        Ok(())
    }

    fn handle_launch(
        &mut self,
        req: Request,
        _args: dap::requests::LaunchRequestArguments,
    ) -> Result<()> {
        match self.mode.clone() {
            RunCommand::Test {
                filter,
                package_path,
                ..
            } => {
                self.send_console(format!(
                    "aptos-dap: test mode — package={}, filter={}",
                    package_path.display(),
                    filter
                ))?;
            },
            RunCommand::Replay {
                txn_id,
                network,
                local_packages,
                named_addresses,
                ..
            } => {
                self.send_console(format!(
                    "aptos-dap: fetching txn {txn_id} from {network}..."
                ))?;
                let replay_txn_info = self
                    .rt
                    .block_on(ReplayTransactionSession::create(&network, txn_id))?;
                self.send_console(format!(
                    "aptos-dap: loaded txn {} (sender: {}, hash: {})",
                    txn_id,
                    replay_txn_info.txn.sender(),
                    replay_txn_info.txn.committed_hash(),
                ))?;
                if !local_packages.is_empty() {
                    self.send_console(format!("aptos-dap: local packages: {:?}", local_packages))?;
                }
                if !named_addresses.is_empty() {
                    self.send_console(format!(
                        "aptos-dap: named addresses: {:?}",
                        named_addresses
                    ))?;
                }
                self.txn_session = Some(replay_txn_info);
            },
        }

        self.server.respond(req.success(ResponseBody::Launch))?;
        Ok(())
    }

    fn handle_set_breakpoints(
        &mut self,
        req: Request,
        args: dap::requests::SetBreakpointsArguments,
    ) -> Result<()> {
        // Canonicalize the path VS Code sends so breakpoints in dependency
        // packages match the absolute paths the source locator produces.
        let raw_path = args.source.path.as_deref().unwrap_or("");
        let source_path = std::path::Path::new(raw_path)
            .canonicalize()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|_| raw_path.to_string());
        let bp_strings: Vec<String> = args
            .breakpoints
            .as_ref()
            .map(|bps| {
                bps.iter()
                    .map(|bp| format!("{}:{}", source_path, bp.line))
                    .collect()
            })
            .unwrap_or_default();

        self.send_console(format_args!("aptos-dap: setBreakpoints: {:?}", bp_strings))?;
        self.pending_breakpoints
            .retain(|bp| !bp.starts_with(&source_path));
        self.pending_breakpoints.extend(bp_strings);

        if let Some(tx) = &self.cmd_tx {
            let _ = tx.send(DapCommand::SetBreakpoints(self.pending_breakpoints.clone()));
        }

        let breakpoints = args
            .breakpoints
            .as_ref()
            .map(|bps| {
                bps.iter()
                    .map(|bp| dap::types::Breakpoint {
                        verified: true,
                        line: Some(bp.line),
                        ..Default::default()
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.server
            .respond(req.success(ResponseBody::SetBreakpoints(
                dap::responses::SetBreakpointsResponse { breakpoints },
            )))?;
        Ok(())
    }

    fn handle_configuration_done(&mut self, req: Request) -> Result<()> {
        self.server
            .respond(req.success(ResponseBody::ConfigurationDone))?;

        match self.mode.clone() {
            RunCommand::Test {
                filter,
                package_path,
                skip_fetch_latest_git_deps,
            } => self.start_test_execution(package_path, filter, skip_fetch_latest_git_deps)?,
            RunCommand::Replay {
                local_packages,
                prebuilt_packages,
                named_addresses,
                skip_fetch_latest_git_deps,
                ..
            } => self.start_replay_execution(
                local_packages,
                prebuilt_packages,
                named_addresses,
                skip_fetch_latest_git_deps,
            )?,
        }

        // VM stops at first instruction. Send breakpoints, then continue to first breakpoint.
        match self.wait_for_vm_event() {
            Some(VmEventResult::Stopped(_)) => {},
            Some(VmEventResult::Terminated(msg)) => {
                self.send_output_and_terminate(msg)?;
                return Ok(());
            },
            None => {
                self.server
                    .send_event(dap::events::Event::Terminated(None))?;
                return Ok(());
            },
        }

        if !self.pending_breakpoints.is_empty() {
            let msg = format!(
                "aptos-dap: setting breakpoints: {:?}",
                self.pending_breakpoints
            );
            self.send_console(msg)?;
            if let Some(tx) = &self.cmd_tx {
                let _ = tx.send(DapCommand::SetBreakpoints(self.pending_breakpoints.clone()));
                let _ = tx.send(DapCommand::Continue);
            }
            match self.wait_for_vm_event() {
                Some(VmEventResult::Stopped(reason)) => {
                    self.send_stop_reason_to_console(&reason)?;
                    self.server
                        .send_event(proto::stopped_event(proto::stop_reason_to_dap(&reason)))?;
                },
                Some(VmEventResult::Terminated(msg)) => {
                    self.send_output_and_terminate(msg)?;
                },
                None => {
                    self.server
                        .send_event(dap::events::Event::Terminated(None))?;
                },
            }
        } else {
            self.send_stop_reason_to_console(&StopReason::Entry)?;
            self.server
                .send_event(proto::stopped_event(StoppedEventReason::Entry))?;
        }
        Ok(())
    }

    fn wait_for_vm_event(&mut self) -> Option<VmEventResult> {
        let event_rx = self.event_rx.as_ref()?;
        match event_rx.recv() {
            Ok(DapEvent::Stopped { reason, vm_state }) => {
                self.vm_stopped_state = Some(vm_state);
                self.stored_variables.clear();
                Some(VmEventResult::Stopped(reason))
            },
            Ok(DapEvent::Terminated { message }) => {
                self.vm_stopped_state = None;
                Some(VmEventResult::Terminated(message))
            },
            Err(_) => {
                self.vm_stopped_state = None;
                Some(VmEventResult::Terminated(None))
            },
        }
    }

    fn send_command_and_wait(&mut self, cmd: DapCommand) -> Result<()> {
        if let Some(tx) = &self.cmd_tx {
            // If send fails the VM thread has exited; wait_for_vm_event below handles that.
            let _ = tx.send(cmd);
        }
        match self.wait_for_vm_event() {
            Some(VmEventResult::Stopped(reason)) => {
                self.send_stop_reason_to_console(&reason)?;
                self.server
                    .send_event(proto::stopped_event(proto::stop_reason_to_dap(&reason)))?;
            },
            Some(VmEventResult::Terminated(msg)) => {
                self.send_output_and_terminate(msg)?;
            },
            None => {
                self.server
                    .send_event(dap::events::Event::Terminated(None))?;
            },
        }
        Ok(())
    }

    fn warn_on_unreachable_breakpoints(&mut self, known_files: &[String]) -> Result<()> {
        let is_replay = matches!(self.mode, RunCommand::Replay { .. });
        let unreachable: Vec<String> = self
            .pending_breakpoints
            .iter()
            .filter(|bp| {
                let file = bp.rsplit_once(':').map(|(f, _)| f).unwrap_or(bp);
                !known_files.iter().any(|f| f == file)
            })
            .cloned()
            .collect();
        for bp in unreachable {
            if is_replay {
                self.send_warning(format_args!(
                    "⚠ Breakpoint at {bp} is unreachable: \
                     no local source package contains this file. \
                     Add the package via useLocalPackages in launch.json",
                ))?;
            } else {
                self.send_warning(format_args!(
                    "⚠ Breakpoint at {bp} is unreachable: \
                     source file not found in the compiled package.",
                ))?;
            }
        }
        Ok(())
    }

    fn handle_threads(&mut self, req: Request) -> Result<()> {
        self.server.respond(req.success(ResponseBody::Threads(
            dap::responses::ThreadsResponse {
                threads: vec![Thread {
                    id: 1,
                    name: "main".to_string(),
                }],
            },
        )))?;
        Ok(())
    }

    fn handle_continue(
        &mut self,
        req: Request,
        _args: dap::requests::ContinueArguments,
    ) -> Result<()> {
        self.server.respond(req.success(ResponseBody::Continue(
            dap::responses::ContinueResponse {
                all_threads_continued: Some(true),
            },
        )))?;
        self.send_command_and_wait(DapCommand::Continue)?;
        Ok(())
    }

    fn handle_next(&mut self, req: Request, _args: dap::requests::NextArguments) -> Result<()> {
        self.server.respond(req.success(ResponseBody::Next))?;
        self.send_command_and_wait(DapCommand::StepOver(1))?;
        Ok(())
    }

    fn handle_step_in(
        &mut self,
        req: Request,
        _args: dap::requests::StepInArguments,
    ) -> Result<()> {
        self.server.respond(req.success(ResponseBody::StepIn))?;
        self.send_command_and_wait(DapCommand::Step(1))?;
        Ok(())
    }

    fn handle_step_out(
        &mut self,
        req: Request,
        _args: dap::requests::StepOutArguments,
    ) -> Result<()> {
        self.server.respond(req.success(ResponseBody::StepOut))?;
        self.send_command_and_wait(DapCommand::StepOut)?;
        Ok(())
    }

    fn handle_stack_trace(
        &mut self,
        req: Request,
        _args: dap::requests::StackTraceArguments,
    ) -> Result<()> {
        let stack_frames = if let Some(vm_state) = &self.vm_stopped_state {
            let (source, line) = vm_state
                .source_location
                .as_deref()
                .map(parse_source_location)
                .unwrap_or((None, 0));
            let mut frames = vec![StackFrame {
                id: 0,
                name: trim_hex_address(&vm_state.function_name),
                source,
                line,
                column: 0,
                ..Default::default()
            }];
            for (i, frame) in vm_state.dap_stack_trace.iter().enumerate() {
                let (source, line) = frame
                    .source_location
                    .as_deref()
                    .map(parse_source_location)
                    .unwrap_or((None, 0));
                frames.push(StackFrame {
                    id: (i + 1) as i64,
                    name: trim_hex_address(&frame.function_name),
                    source,
                    line,
                    column: 0,
                    ..Default::default()
                });
            }
            frames
        } else if let Some(session) = &self.txn_session {
            let frame_name = replay::entry_function_name(session.txn.payload());
            vec![StackFrame {
                id: 0,
                name: frame_name,
                source: None,
                line: 0,
                column: 0,
                ..Default::default()
            }]
        } else {
            vec![]
        };
        let total = stack_frames.len() as i64;
        self.server.respond(req.success(ResponseBody::StackTrace(
            dap::responses::StackTraceResponse {
                stack_frames,
                total_frames: Some(total),
            },
        )))?;
        Ok(())
    }

    fn handle_scopes(&mut self, req: Request, args: dap::requests::ScopesArguments) -> Result<()> {
        let frame_id = args.frame_id;

        let mut scopes = vec![];
        if frame_id == 0 && self.txn_session.is_some() {
            scopes.push(Scope {
                name: "Transaction Info".to_string(),
                presentation_hint: None,
                variables_reference: replay::SCOPE_TRANSACTION_INFO,
                named_variables: None,
                indexed_variables: None,
                expensive: false,
                source: None,
                line: None,
                column: None,
                end_line: None,
                end_column: None,
            });
        }
        if self.vm_stopped_state.is_some() {
            scopes.push(Scope {
                name: "Locals".to_string(),
                presentation_hint: Some(ScopePresentationhint::Locals),
                variables_reference: frame_locals_ref_id(frame_id),
                named_variables: None,
                indexed_variables: None,
                expensive: false,
                source: None,
                line: None,
                column: None,
                end_line: None,
                end_column: None,
            });
        }
        self.server.respond(
            req.success(ResponseBody::Scopes(dap::responses::ScopesResponse {
                scopes,
            })),
        )?;
        Ok(())
    }

    // VSCode calls "variables" with a variablesReference to fetch the contents
    // of a scope or an expandable variable. The reference ID encodes what to show:
    //
    //   >= STRUCT_FIELDS_BASE (100k) — children of an expanded struct/vector,
    //                                  looked up in self.expanded_vars
    //   == SCOPE_TRANSACTION_INFO    — replay-mode transaction metadata
    //   == SCOPE_ARGUMENTS           — replay-mode transaction arguments
    //   >= SCOPE_LOCALS_BASE (1000)  — locals for stack frame (ref - base = frame index)
    fn handle_variables(
        &mut self,
        req: Request,
        args: dap::requests::VariablesArguments,
    ) -> Result<()> {
        let variable_ref_id = args.variables_reference;
        let variables = match variable_ref_id {
            SCOPE_TRANSACTION_INFO => self.transaction_info_variables(),
            _ => self
                .stored_variables
                .get_variables(self.vm_stopped_state.as_ref(), variable_ref_id),
        };
        self.server.respond(req.success(ResponseBody::Variables(
            dap::responses::VariablesResponse { variables },
        )))?;
        Ok(())
    }

    fn handle_disconnect(
        &mut self,
        req: Request,
        _args: dap::requests::DisconnectArguments,
    ) -> Result<()> {
        self.cmd_tx = None;
        self.event_rx = None;
        if let Some(handle) = self.vm_thread.take() {
            let _ = handle.join();
        }
        self.server.respond(req.success(ResponseBody::Disconnect))?;
        Ok(())
    }

    fn send_console(&mut self, msg: impl Display) -> Result<()> {
        self.send_output(OutputEventCategory::Console, msg)
    }

    fn send_warning(&mut self, msg: impl Display) -> Result<()> {
        self.send_output(OutputEventCategory::Stderr, msg)
    }

    fn send_output(&mut self, category: OutputEventCategory, msg: impl Display) -> Result<()> {
        self.server
            .send_event(dap::events::Event::Output(OutputEventBody {
                category: Some(category),
                output: format!("{msg}\n"),
                ..Default::default()
            }))?;
        Ok(())
    }

    fn send_stop_reason_to_console(&mut self, reason: &StopReason) -> Result<()> {
        if let Some(state) = &self.vm_stopped_state {
            let location = state
                .source_location
                .as_deref()
                .unwrap_or("unknown location");
            let msg = match reason {
                StopReason::Breakpoint(name) => {
                    format!("Breakpoint hit: {name}")
                },
                StopReason::Entry => {
                    format!("Stopped at entry: {} at {location}", state.function_name)
                },
                StopReason::Step => {
                    format!("Stepped to: {} at {location}", state.function_name)
                },
            };
            self.send_console(msg)?;
        }
        Ok(())
    }

    fn send_output_and_terminate(&mut self, message: Option<String>) -> Result<()> {
        if let Some(msg) = message {
            self.server
                .send_event(dap::events::Event::Output(OutputEventBody {
                    category: Some(OutputEventCategory::Stderr),
                    output: format!("{msg}\n"),
                    ..Default::default()
                }))?;
        }
        self.server
            .send_event(dap::events::Event::Terminated(None))?;
        Ok(())
    }
}

enum VmEventResult {
    Stopped(StopReason),
    Terminated(Option<String>),
}
