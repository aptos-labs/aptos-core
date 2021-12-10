// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::context::Context;
use lsp_server::Request;
use lsp_types::{CompletionItem, CompletionItemKind, CompletionParams, Position};
use move_command_line_common::files::FileHash;
use move_compiler::{
    diagnostics::Diagnostic,
    parser::{
        keywords::{BUILTINS, CONTEXTUAL_KEYWORDS, KEYWORDS},
        lexer::{Lexer, Tok},
    },
};
use std::collections::HashSet;

/// Constructs an `lsp_types::CompletionItem` with the given `label` and `kind`.
fn completion_item(label: &str, kind: CompletionItemKind) -> CompletionItem {
    CompletionItem {
        label: label.to_owned(),
        kind: Some(kind),
        ..Default::default()
    }
}

/// Return a list of completion items corresponding to each one of Move's keywords.
///
/// Currently, this does not filter keywords out based on whether they are valid at the completion
/// request's cursor position, but in the future it ought to. For example, this function returns
/// all specification language keywords, but in the future it should be modified to only do so
/// within a spec block.
fn keywords() -> Vec<CompletionItem> {
    KEYWORDS
        .iter()
        .chain(CONTEXTUAL_KEYWORDS.iter())
        .map(|label| {
            let kind = if label == &"copy" || label == &"move" {
                CompletionItemKind::Operator
            } else {
                CompletionItemKind::Keyword
            };
            completion_item(label, kind)
        })
        .collect()
}

/// Return a list of completion items corresponding to each one of Move's builtin functions.
fn builtins() -> Vec<CompletionItem> {
    BUILTINS
        .iter()
        .map(|label| completion_item(label, CompletionItemKind::Function))
        .collect()
}

/// Lexes the Move source file at the given path and returns a list of completion items
/// corresponding to the non-keyword identifiers therein. If lexing produces diagnostics, those are
/// also returned.
///
/// Currently, this does not perform semantic analysis to determine whether the identifiers
/// returned are valid at the request's cursor position. However, this list of identifiers is akin
/// to what editors like Visual Studio Code would provide as completion items if this language
/// server did not initialize with a response indicating it's capable of providing completions. In
/// the future, the server should be modified to return semantically valid completion items, not
/// simple textual suggestions.
fn identifiers(buffer: &str) -> (Vec<CompletionItem>, Vec<Diagnostic>) {
    let mut lexer = Lexer::new(buffer, FileHash::new(buffer));
    if let Err(diagnostic) = lexer.advance() {
        return (vec![], vec![diagnostic]);
    }

    let mut ids = HashSet::new();
    let mut diagnostics = vec![];
    while lexer.peek() != Tok::EOF {
        // Some tokens, such as "phantom", are contextual keywords that are only reserved in
        // certain contexts. Since for now this language server doesn't analyze semantic context,
        // tokens such as "phantom" are always present in keyword suggestions. To avoid displaying
        // these keywords to the user twice in the case that the token "phantom" is present in the
        // source program (once as a keyword, and once as an identifier), we filter out any
        // identifier token that has the same text as a keyword.
        if lexer.peek() == Tok::Identifier && !KEYWORDS.contains(&lexer.content()) {
            // The completion item kind "text" indicates the item is not based on any semantic
            // context of the request cursor's position.
            ids.insert(lexer.content());
        }
        if let Err(diagnostic) = lexer.advance() {
            diagnostics.push(diagnostic)
        }
    }

    // The completion item kind "text" indicates that the item is based on simple textual matching,
    // not any deeper semantic analysis.
    let items = ids
        .iter()
        .map(|label| completion_item(label, CompletionItemKind::Text))
        .collect();
    (items, diagnostics)
}

/// Returns the token corresponding to the "trigger character" that precedes the user's cursor,
/// if it is one of `.`, `:`, or `::`. Otherwise, returns `None`.
fn get_cursor_token(buffer: &str, position: &Position) -> Option<Tok> {
    // If the cursor is at the start of a new line, it cannot be preceded by a trigger character.
    if position.character == 0 {
        return None;
    }

    let line = match buffer.lines().nth(position.line as usize) {
        Some(line) => line,
        None => return None, // Our buffer does not contain the line, and so must be out of date.
    };
    match line.chars().nth(position.character as usize - 1) {
        Some('.') => Some(Tok::Period),
        Some(':') => {
            if position.character > 1
                && line.chars().nth(position.character as usize - 2) == Some(':')
            {
                Some(Tok::ColonColon)
            } else {
                Some(Tok::Colon)
            }
        }
        _ => None,
    }
}

/// Sends the given connection a response to a completion request.
///
/// The completions returned depend upon where the user's cursor is positioned.
pub fn on_completion_request(context: &Context, request: &Request) {
    let parameters = serde_json::from_value::<CompletionParams>(request.params.clone())
        .expect("could not deserialize request");

    let path = parameters.text_document_position.text_document.uri.path();
    let buffer = context.files.get(path);
    if buffer.is_none() {
        eprintln!("Could not read '{}' when handling completion request", path);
    }

    // The completion items we provide depend upon where the user's cursor is positioned.
    let cursor = buffer
        .map(|buf| get_cursor_token(buf, &parameters.text_document_position.position))
        .flatten();

    let mut items = vec![];
    match cursor {
        Some(Tok::Colon) => {
            // If the user's cursor is positioned after a single `:`, do not provide any completion
            // items at all -- this is a "mis-fire" of the "trigger character" `:`.
            return;
        }
        Some(Tok::Period) | Some(Tok::ColonColon) => {
            // `.` or `::` must be followed by identifiers, which are added to the completion items
            // below.
        }
        _ => {
            // If the user's cursor is positioned anywhere other than following a `.`, `:`, or `::`,
            // offer them Move's keywords, operators, and builtins as completion items.
            items.extend_from_slice(&keywords());
            items.extend_from_slice(&builtins());
        }
    }

    if let Some(buffer) = &buffer {
        let (identifiers, diagnostics) = identifiers(buffer);
        items.extend_from_slice(&identifiers);
        if !diagnostics.is_empty() {
            // TODO: A language server is capable of surfacing diagnostics to its client, but to do so
            // would require us to translate move_compiler's diagnostic location (a private member
            // represented as a byte offset) to a line and column number. The move_compiler::diagnostics
            // module currently only allows for printing a set of diagnostics to stderr, but that could
            // be changed. (If this server *was* to surface diagnostics, it would also be responsible
            // for notifying the client when the diagnostics were fixed and ought to be cleared. It
            // would be a poor user experience to only surface and clear diagnostics here, when
            // responding to a completion request.)
            eprintln!(
                "encountered {} diagnostic(s) while lexing '{}'",
                diagnostics.len(),
                path
            );
        }
    }

    let result = serde_json::to_value(items).expect("could not serialize response");
    let response = lsp_server::Response::new_ok(request.id.clone(), result);
    context
        .connection
        .sender
        .send(lsp_server::Message::Response(response))
        .expect("could not send response");
}
