// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use lsp_server::{Connection, Message, Notification, Request, Response};
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
    eprintln!(
        "Starting language server '{}' communicating via stdio...",
        std::env::current_exe().unwrap().to_string_lossy()
    );

    let (connection, _) = Connection::stdio();
    let capabilities = serde_json::to_value(lsp_types::ServerCapabilities {
        text_document_sync: None,
        selection_range_provider: None,
        hover_provider: None,
        completion_provider: None,
        signature_help_provider: None,
        definition_provider: Some(lsp_types::OneOf::Left(true)),
        type_definition_provider: None,
        implementation_provider: None,
        references_provider: None,
        document_highlight_provider: None,
        document_symbol_provider: None,
        workspace_symbol_provider: None,
        code_action_provider: None,
        code_lens_provider: None,
        document_formatting_provider: None,
        document_range_formatting_provider: None,
        document_on_type_formatting_provider: None,
        rename_provider: None,
        document_link_provider: None,
        color_provider: None,
        folding_range_provider: None,
        declaration_provider: None,
        execute_command_provider: None,
        workspace: None,
        call_hierarchy_provider: None,
        semantic_tokens_provider: None,
        moniker_provider: None,
        linked_editing_range_provider: None,
        experimental: None,
    })
    .expect("could not serialize server capabilities");
    connection
        .initialize(capabilities)
        .expect("could not initialize the connection");

    loop {
        match connection.receiver.recv() {
            Ok(message) => match message {
                Message::Request(request) => on_request(&request),
                Message::Response(response) => on_response(&response),
                Message::Notification(notification) => on_notification(&notification),
            },
            Err(error) => {
                eprintln!("error: {:?}", error);
                break;
            }
        }
    }
}

fn on_request(request: &Request) {
    eprintln!("request: {:?}", request);
    todo!("handle request from client");
}

fn on_response(response: &Response) {
    eprintln!("response: {:?}", response);
    todo!("handle response from client");
}

fn on_notification(notification: &Notification) {
    eprintln!("notification: {:?}", notification);
    todo!("handle notification from client");
}
