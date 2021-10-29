// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    notification::Notification as _, request::Request as _, CompletionOptions, SaveOptions,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    WorkDoneProgressOptions,
};
use move_analyzer::{
    completion::on_completion_request,
    context::Context,
    vfs::{on_text_document_sync_notification, VirtualFileSystem},
};
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(name = "move-analyzer", about = "A language server for Move")]
struct Options {}

fn main() {
    // For now, move-analyzer only responds to options built-in to structopt,
    // such as `--help` or `--version`.
    Options::from_args();

    // stdio is used to communicate Language Server Protocol requests and responses.
    // stderr is used for logging (and, when Visual Studio Code is used to communicate with this
    // server, it captures this output in a dedicated "output channel").
    let exe = std::env::current_exe()
        .unwrap()
        .to_string_lossy()
        .to_string();
    eprintln!(
        "Starting language server '{}' communicating via stdio...",
        exe
    );

    let (connection, io_threads) = Connection::stdio();
    let mut context = Context {
        connection,
        files: VirtualFileSystem::default(),
    };
    let capabilities = serde_json::to_value(lsp_types::ServerCapabilities {
        // The server receives notifications from the client as users open, close,
        // and modify documents.
        text_document_sync: Some(TextDocumentSyncCapability::Options(
            TextDocumentSyncOptions {
                open_close: Some(true),
                // TODO: We request that the language server client send us the entire text of any
                // files that are modified. We ought to use the "incremental" sync kind, which would
                // have clients only send us what has changed and where, thereby requiring far less
                // data be sent "over the wire." However, to do so, our language server would need
                // to be capable of applying deltas to its view of the client's open files. See the
                // 'move_analyzer::vfs' module for details.
                change: Some(TextDocumentSyncKind::Full),
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
        hover_provider: None,
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
        }),
        ..Default::default()
    })
    .expect("could not serialize server capabilities");
    context
        .connection
        .initialize(capabilities)
        .expect("could not initialize the connection");

    loop {
        match context.connection.receiver.recv() {
            Ok(message) => match message {
                Message::Request(request) => on_request(&context, &request),
                Message::Response(response) => on_response(&context, &response),
                Message::Notification(notification) => {
                    match notification.method.as_str() {
                        lsp_types::notification::Exit::METHOD => break,
                        lsp_types::notification::Cancel::METHOD => {
                            // TODO: Currently the server does not implement request cancellation.
                            // It ought to, especially once it begins processing requests that may
                            // take a long time to respond to.
                        }
                        _ => on_notification(&mut context, &notification),
                    }
                }
            },
            Err(error) => {
                eprintln!("error: {:?}", error);
                break;
            }
        }
    }

    io_threads.join().expect("I/O threads could not finish");
    eprintln!("Shut down language server '{}'.", exe);
}

fn on_request(context: &Context, request: &Request) {
    match request.method.as_str() {
        lsp_types::request::Completion::METHOD => on_completion_request(context, request),
        _ => todo!("handle request '{}' from client", request.method),
    }
}

fn on_response(_context: &Context, _response: &Response) {
    todo!("handle response from client");
}

fn on_notification(context: &mut Context, notification: &Notification) {
    match notification.method.as_str() {
        lsp_types::notification::DidOpenTextDocument::METHOD
        | lsp_types::notification::DidChangeTextDocument::METHOD
        | lsp_types::notification::DidSaveTextDocument::METHOD
        | lsp_types::notification::DidCloseTextDocument::METHOD => {
            on_text_document_sync_notification(&mut context.files, notification)
        }
        _ => todo!("handle notification '{}' from client", notification.method),
    }
}
