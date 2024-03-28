// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_move_analyzer::{
    analyzer_handler::ConvertLoc,
    completion,
    context::{Context, FileDiags},
    goto_definition, hover, inlay_hints,
    inlay_hints::*,
    move_generate_spec_file::on_generate_spec_file,
    move_generate_spec_sel::on_generate_spec_sel,
    multiproject::MultiProject,
    references, symbols,
    utils::*,
};
use clap::Parser;
use crossbeam::channel::{bounded, select, Sender};
use log::{Level, Metadata, Record};
use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    notification::Notification as _, request::Request as _, CompletionOptions,
    HoverProviderCapability, OneOf, SaveOptions, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncOptions, WorkDoneProgressOptions,
};
use move_command_line_common::files::FileHash;
use move_compiler::{diagnostics::Diagnostics, PASS_TYPING};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use url::Url;

struct SimpleLogger;
impl log::Log for SimpleLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            eprintln!("{} - {}", record.level(), record.args());
        }
    }

    fn flush(&self) {}
}
const LOGGER: SimpleLogger = SimpleLogger;

pub fn init_log() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(log::LevelFilter::Trace))
        .unwrap()
}

#[derive(Parser)]
#[clap(author, version, about)]
struct Options {}

fn main() {
    // For now, aptos-move-analyzer only responds to options built-in to clap,
    // such as `--help` or `--version`.
    Options::parse();
    init_log();
    // stdio is used to communicate Language Server Protocol requests and responses.
    // stderr is used for logging (and, when Visual Studio Code is used to communicate with this
    // server, it captures this output in a dedicated "output channel").
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .to_string();
    log::info!(
        "Starting language server '{}' communicating via stdio...",
        exe
    );

    let (connection, io_threads) = Connection::stdio();
    let mut context = Context {
        projects: MultiProject::new(),
        connection,
        diag_version: FileDiags::new(),
    };

    let (id, _client_response) = context
        .connection
        .initialize_start()
        .expect("could not start connection initialization");

    let capabilities = serde_json::to_value(lsp_types::ServerCapabilities {
        // The server receives notifications from the client as users open, close,
        // and modify documents.
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                change: Some(TextDocumentSyncKind::FULL),
                will_save: None,
                will_save_wait_until: None,
                save: Some(
                    SaveOptions {
                        include_text: Some(true),
                    }
                    .into(),
                ),
            },
        )),
        selection_range_provider: None,
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        // The server provides completions as a user is typing.
        completion_provider: Some(CompletionOptions {
            resolve_provider: None,
            // In Move, `foo::` and `foo.` should trigger completion suggestions for after
            // the `:` or `.`
            // (Trigger characters are just that: characters, such as `:`, and not sequences of
            // characters, such as `::`. So when the language server encounters a completion
            // request, it checks whether completions are being requested for `foo:`, and returns no
            // completions in that case.)
            trigger_characters: Some(vec![":".to_string(), ".".to_string()]),
            all_commit_characters: None,
            work_done_progress_options: WorkDoneProgressOptions {
                work_done_progress: None,
            },
            completion_item: None,
        }),
        definition_provider: Some(OneOf::Left(true)),
        // type_definition_provider: Some(TypeDefinitionProviderCapability::Simple(
        //     true,
        // )),
        references_provider: Some(OneOf::Left(true)),
        document_symbol_provider: Some(OneOf::Left(true)),
        ..Default::default()
    })
    .expect("could not serialize server capabilities");

    context
        .connection
        .initialize_finish(
            id,
            serde_json::json!({
                "capabilities": capabilities,
            }),
        )
        .expect("could not finish connection initialization");
    let (diag_sender, diag_receiver) = bounded::<(PathBuf, Diagnostics)>(1);
    let diag_sender = Arc::new(Mutex::new(diag_sender));
    let mut inlay_hints_config = InlayHintsConfig::default();
    loop {
        select! {
            recv(diag_receiver) -> message => {
                match message {
                    Ok ((mani ,x)) => {
                        send_diag(&mut context,mani,x);
                    }
                    Err(error) => log::error!("IDE diag message error: {:?}", error),
                }
            },
            recv(context.connection.receiver) -> message => {
                context.projects.try_reload_projects(&context.connection);
                match message {
                    Ok(Message::Request(request)) => on_request(&mut context, &request , &mut inlay_hints_config),
                    Ok(Message::Response(response)) => on_response(&context, &response),
                    Ok(Message::Notification(notification)) => {
                        match notification.method.as_str() {
                            lsp_types::notification::Exit::METHOD => break,
                            lsp_types::notification::Cancel::METHOD => {
                                // TODO: Currently the server does not implement request cancellation.
                                // It ought to, especially once it begins processing requests that may
                                // take a long time to respond to.
                            }
                            _ => on_notification(&mut context, &notification, diag_sender.clone()),
                        }
                    }
                    Err(error) => log::error!("IDE message error: {:?}", error),
                }
            }
        };
    }

    io_threads.join().expect("I/O threads could not finish");
    log::error!("Shut down language server '{}'.", exe);
}

fn on_request(context: &mut Context, request: &Request, inlay_hints_config: &mut InlayHintsConfig) {
    match request.method.as_str() {
        lsp_types::request::GotoDefinition::METHOD => {
            goto_definition::on_go_to_def_request(context, request);
        },
        lsp_types::request::References::METHOD => {
            references::on_references_request(context, request);
        },
        lsp_types::request::HoverRequest::METHOD => {
            hover::on_hover_request(context, request);
        },
        lsp_types::request::Completion::METHOD => {
            completion::on_completion_request(context, request);
        },
        lsp_types::request::InlayHintRequest::METHOD => {
            inlay_hints::on_inlay_hints(context, request, inlay_hints_config);
        },
        lsp_types::request::DocumentSymbolRequest::METHOD => {
            symbols::on_document_symbol_request(context, request);
        },
        "move/generate/spec/file" => {
            on_generate_spec_file(context, request, true);
        },
        "move/generate/spec/sel" => {
            on_generate_spec_sel(context, request);
        },
        "move/lsp/client/inlay_hints/config" => {
            let parameters = serde_json::from_value::<InlayHintsConfig>(request.params.clone())
                .expect("could not deserialize inlay hints request");
            log::info!("call inlay_hints config {:?}", parameters);
            *inlay_hints_config = parameters;
        },
        _ => {
            log::error!("unsupported request: '{}' from client", request.method)
        },
    }
}

fn on_response(_context: &Context, _response: &Response) {
    log::info!("handle response from client");
}

type DiagSender = Arc<Mutex<Sender<(PathBuf, Diagnostics)>>>;

fn report_diag(context: &mut Context, fpath: PathBuf) {
    let proj = match context.projects.get_project(&fpath) {
        Some(x) => x,
        None => {
            log::error!("project not found:{:?}", fpath.as_path());
            return;
        },
    };
    log::info!("report_diag -------------");

    let mut result: HashMap<Url, Vec<lsp_types::Diagnostic>> = HashMap::new();
    let diag_err = proj.err_diags.clone();
    let tokens: Vec<&str> = diag_err.as_str().split("error").collect();
    for token in tokens {
        if token.lines().count() < 3 {
            continue;
        }
        let err_msg = token.lines().next().unwrap_or_default();
        let loc_str = match token.lines().nth(1) {
            Some(str) => str.to_string(),
            None => "".to_string(),
        };

        let mut file_path = "";
        let mut pos = lsp_types::Position::default();
        if let Some(start_idx) = loc_str.find('/') {
            if let Some(end_idx) = loc_str.find(':') {
                if start_idx <= end_idx {
                    file_path = &loc_str[start_idx..end_idx];
                    let line = loc_str[end_idx..].split(':').nth(1).unwrap_or_default();
                    let col = loc_str[end_idx..].split(':').nth(2).unwrap_or_default();
                    let line_num = if line.parse::<u32>().unwrap() > 0 {
                        line.parse::<u32>().unwrap() - 1
                    } else {
                        line.parse::<u32>().unwrap()
                    };
                    let col_num = if col.parse::<u32>().unwrap() > 0 {
                        col.parse::<u32>().unwrap() - 1
                    } else {
                        col.parse::<u32>().unwrap()
                    };
                    pos = lsp_types::Position::new(line_num, col_num);
                }
            }
        }

        if file_path.contains("aptos-move/") || file_path.is_empty() {
            continue;
        }

        let mut code_str = "".to_string();
        for line_idx in 2..token.lines().count() {
            code_str.push_str(token.lines().nth(line_idx).unwrap());
        }
        let d = lsp_types::Diagnostic {
            range: lsp_types::Range {
                start: pos,
                end: pos,
            },
            severity: Some(lsp_types::DiagnosticSeverity::ERROR),
            message: format!("{}\n{}{:?}", err_msg, code_str, "".to_string()),
            ..Default::default()
        };
        let url = url::Url::from_file_path(PathBuf::from(file_path).as_path()).unwrap();
        result.insert(url, vec![d]);
    }
    for (k, v) in result.into_iter() {
        let ds = lsp_types::PublishDiagnosticsParams::new(k.clone(), v, None);
        context
            .connection
            .sender
            .send(lsp_server::Message::Notification(Notification {
                method: lsp_types::notification::PublishDiagnostics::METHOD.to_string(),
                params: serde_json::to_value(ds).unwrap(),
            }))
            .unwrap();
    }
}

fn on_notification(context: &mut Context, notification: &Notification, diag_sender: DiagSender) {
    fn update_defs_on_changed(context: &mut Context, fpath: PathBuf, content: String) {
        let file_hash = FileHash::new(content.as_str());
        context.projects.update_defs(fpath.clone(), content.clone());
        context
            .projects
            .hash_file
            .as_ref()
            .borrow_mut()
            .update(fpath.clone(), file_hash);
        context
            .projects
            .file_line_mapping
            .as_ref()
            .borrow_mut()
            .update(fpath.clone(), content);
        report_diag(context, fpath);
    }

    match notification.method.as_str() {
        lsp_types::notification::DidSaveTextDocument::METHOD => {
            use lsp_types::DidSaveTextDocumentParams;
            let parameters =
                serde_json::from_value::<DidSaveTextDocumentParams>(notification.params.clone())
                    .expect("could not deserialize DidSaveTextDocumentParams request");
            let fpath = parameters.text_document.uri.to_file_path().unwrap();
            let fpath = path_concat(&std::env::current_dir().unwrap(), &fpath);
            let content = std::fs::read_to_string(fpath.as_path());
            let content = match content {
                Ok(x) => x,
                Err(err) => {
                    log::error!("read file failed,err:{:?}", err);
                    return;
                },
            };
            update_defs_on_changed(context, fpath.clone(), content);
            make_diag(context, diag_sender, fpath);
        },
        lsp_types::notification::DidChangeTextDocument::METHOD => {
            use lsp_types::DidChangeTextDocumentParams;
            let parameters =
                serde_json::from_value::<DidChangeTextDocumentParams>(notification.params.clone())
                    .expect("could not deserialize DidChangeTextDocumentParams request");
            let fpath = parameters.text_document.uri.to_file_path().unwrap();
            let fpath = path_concat(&std::env::current_dir().unwrap(), &fpath);
            update_defs_on_changed(
                context,
                fpath,
                parameters.content_changes.last().unwrap().text.clone(),
            );
        },

        lsp_types::notification::DidOpenTextDocument::METHOD => {
            use lsp_types::DidOpenTextDocumentParams;
            let parameters =
                serde_json::from_value::<DidOpenTextDocumentParams>(notification.params.clone())
                    .expect("could not deserialize DidOpenTextDocumentParams request");
            let fpath = parameters.text_document.uri.to_file_path().unwrap();
            let fpath = path_concat(&std::env::current_dir().unwrap(), &fpath);
            let (mani, _) = match discover_manifest_and_kind(&fpath) {
                Some(x) => x,
                None => {
                    log::error!("not move project.");
                    send_not_project_file_error(context, fpath, true);
                    return;
                },
            };
            match context.projects.get_project(&fpath) {
                Some(_) => {
                    if std::fs::read_to_string(fpath.as_path()).is_ok() {
                        // update_defs_on_changed(context, fpath.clone(), x);
                    };
                    return;
                },
                None => {
                    log::error!("project '{:?}' not found try load.", fpath.as_path());
                },
            };
            let p = match context.projects.load_projects(&context.connection, &mani) {
                anyhow::Result::Ok(x) => x,
                anyhow::Result::Err(e) => {
                    log::error!("load project failed,err:{:?}", e);
                    return;
                },
            };

            context.projects.insert_project(p);
            make_diag(context, diag_sender, fpath.clone());
            report_diag(context, fpath);
        },
        lsp_types::notification::DidCloseTextDocument::METHOD => {
            use lsp_types::DidCloseTextDocumentParams;
            let parameters =
                serde_json::from_value::<DidCloseTextDocumentParams>(notification.params.clone())
                    .expect("could not deserialize DidCloseTextDocumentParams request");
            let fpath = parameters.text_document.uri.to_file_path().unwrap();
            let fpath = path_concat(&std::env::current_dir().unwrap(), &fpath);
            let (_, _) = match discover_manifest_and_kind(&fpath) {
                Some(x) => x,
                None => {
                    log::error!("not move project.");
                    send_not_project_file_error(context, fpath, false);
                    return;
                },
            };
        },

        _ => {},
    }
}

fn get_package_compile_diagnostics(pkg_path: &Path) -> Result<Diagnostics> {
    use anyhow::*;
    use move_package::compilation::build_plan::BuildPlan;
    use tempfile::tempdir;
    let build_config = move_package::BuildConfig {
        test_mode: true,
        install_dir: Some(tempdir().unwrap().path().to_path_buf()),
        skip_fetch_latest_git_deps: true,
        ..Default::default()
    };
    // resolution graph diagnostics are only needed for CLI commands so ignore them by passing a
    // vector as the writer
    let resolution_graph = build_config.resolution_graph_for_package(pkg_path, &mut Vec::new())?;
    let build_plan = BuildPlan::create(resolution_graph)?;
    let mut diagnostics = None;
    let compile_cfg: move_package::CompilerConfig = Default::default();
    build_plan.compile_with_driver(
        &mut std::io::sink(),
        &compile_cfg,
        |compiler| {
            let (_, compilation_result) = compiler.run::<PASS_TYPING>()?;
            match compilation_result {
                std::result::Result::Ok(_) => {},
                std::result::Result::Err(diags) => {
                    log::error!("get_package_compile_diagnostics compilate failed");
                    diagnostics = Some(diags);
                },
            };
            Ok(Default::default())
        },
        |_compiler| Ok(Default::default()),
    )?;
    match diagnostics {
        Some(x) => Ok(x),
        None => Ok(Default::default()),
    }
}

fn make_diag(context: &Context, diag_sender: DiagSender, fpath: PathBuf) {
    let (mani, _) = match aptos_move_analyzer::utils::discover_manifest_and_kind(fpath.as_path()) {
        Some(x) => x,
        None => {
            log::error!("manifest not found.");
            return;
        },
    };
    match context.projects.get_project(&fpath) {
        Some(x) => {
            if !x.load_ok() {
                return;
            }
        },
        None => return,
    };
    std::thread::spawn(move || {
        let x = match get_package_compile_diagnostics(mani.as_path()) {
            Ok(x) => x,
            Err(err) => {
                log::error!("get_package_compile_diagnostics failed,err:{:?}", err);
                return;
            },
        };
        diag_sender.lock().unwrap().send((mani, x)).unwrap();
    });
}

fn send_not_project_file_error(context: &mut Context, fpath: PathBuf, is_open: bool) {
    let url = url::Url::from_file_path(fpath.as_path()).unwrap();
    let content = std::fs::read_to_string(fpath.as_path()).unwrap_or_else(|_| "".to_string());
    let lines: Vec<_> = content.lines().collect();
    let last_line = lines.len();
    let last_col = lines.last().map(|x| (*x).len()).unwrap_or(1);
    let ds = lsp_types::PublishDiagnosticsParams::new(
        url,
        if is_open {
            vec![lsp_types::Diagnostic {
                range: lsp_types::Range {
                    start: lsp_types::Position {
                        line: 0,
                        character: 0,
                    },
                    end: lsp_types::Position {
                        line: last_line as u32,
                        character: last_col as u32,
                    },
                },
                message: "This file doesn't belong to a move project.\nMaybe a build artifact???"
                    .to_string(),
                ..Default::default()
            }]
        } else {
            vec![]
        },
        None,
    );
    context
        .connection
        .sender
        .send(lsp_server::Message::Notification(Notification {
            method: lsp_types::notification::PublishDiagnostics::METHOD.to_string(),
            params: serde_json::to_value(ds).unwrap(),
        }))
        .unwrap();
}

fn send_diag(context: &mut Context, mani: PathBuf, x: Diagnostics) {
    let mut result: HashMap<Url, Vec<lsp_types::Diagnostic>> = HashMap::new();
    for x in x.into_codespan_format() {
        let (s, msg, (loc, m), _, notes) = x;
        if let Some(r) = context.projects.convert_loc_range(&loc) {
            let url = url::Url::from_file_path(r.path.as_path()).unwrap();
            let d = lsp_types::Diagnostic {
                range: r.mk_location().range,
                severity: Some(match s {
                    codespan_reporting::diagnostic::Severity::Bug => {
                        lsp_types::DiagnosticSeverity::ERROR
                    },
                    codespan_reporting::diagnostic::Severity::Error => {
                        lsp_types::DiagnosticSeverity::ERROR
                    },
                    codespan_reporting::diagnostic::Severity::Warning => {
                        lsp_types::DiagnosticSeverity::WARNING
                    },
                    codespan_reporting::diagnostic::Severity::Note => {
                        lsp_types::DiagnosticSeverity::INFORMATION
                    },
                    codespan_reporting::diagnostic::Severity::Help => {
                        lsp_types::DiagnosticSeverity::HINT
                    },
                }),
                message: format!(
                    "{}\n{}{:?}",
                    msg,
                    m,
                    if !notes.is_empty() {
                        format!(" {:?}", notes)
                    } else {
                        "".to_string()
                    }
                ),
                ..Default::default()
            };
            if let Some(a) = result.get_mut(&url) {
                a.push(d);
            } else {
                result.insert(url, vec![d]);
            };
        }
    }
    // update version.
    for (k, v) in result.iter() {
        context.diag_version.update(&mani, k, v.len());
    }
    context.diag_version.with_manifest(&mani, |x| {
        for (old, v) in x.iter() {
            if !result.contains_key(old) && *v > 0 {
                result.insert(old.clone(), vec![]);
            }
        }
    });
    for (k, x) in result.iter() {
        if x.is_empty() {
            context.diag_version.update(&mani, k, 0);
        }
    }
    for (k, v) in result.into_iter() {
        let ds = lsp_types::PublishDiagnosticsParams::new(k.clone(), v, None);
        context
            .connection
            .sender
            .send(lsp_server::Message::Notification(Notification {
                method: lsp_types::notification::PublishDiagnostics::METHOD.to_string(),
                params: serde_json::to_value(ds).unwrap(),
            }))
            .unwrap();
    }
}
