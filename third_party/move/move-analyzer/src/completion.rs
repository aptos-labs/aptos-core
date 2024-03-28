// Copyright (c) The BitsLab.MoveBit Contributors
// SPDX-License-Identifier: Apache-2.0
use crate::{context::*, utils::path_concat};
use lsp_server::{Request, *};
use lsp_types::{CompletionItem, CompletionItemKind, CompletionParams, Position};
use move_command_line_common::files::FileHash;
use move_compiler::parser::{
    keywords::{BUILTINS, CONTEXTUAL_KEYWORDS, KEYWORDS, PRIMITIVE_TYPES},
    lexer::{Lexer, Tok},
};
use move_model::model::GlobalEnv;
use std::vec;

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
        .chain(PRIMITIVE_TYPES.iter())
        .map(|label| {
            let kind = if label == &"copy" || label == &"move" {
                CompletionItemKind::OPERATOR
            } else {
                CompletionItemKind::KEYWORD
            };
            completion_item(label, kind)
        })
        .collect()
}

/// Return a list of completion items of Move's primitive types
fn primitive_types() -> Vec<CompletionItem> {
    PRIMITIVE_TYPES
        .iter()
        .map(|label| completion_item(label, CompletionItemKind::KEYWORD))
        .collect()
}

/// Return a list of completion items corresponding to each one of Move's builtin functions.
fn builtins() -> Vec<CompletionItem> {
    BUILTINS
        .iter()
        .map(|label| completion_item(label, CompletionItemKind::FUNCTION))
        .collect()
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
    log::info!("cursor line: {}", line);
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
        },
        _ => None,
    }
}

pub fn get_cursor_line(buffer: &str, position: &Position) -> Option<String> {
    let line = match buffer.lines().nth(position.line as usize) {
        Some(line) => line,
        None => return None, // Our buffer does not contain the line, and so must be out of date.
    };
    Some(line.to_string())
}

fn truncate_from_last_space(input: String) -> String {
    if let Some(last_space_index) = input.rfind(' ') {
        let truncated_part = &input[last_space_index + 1..];
        String::from(truncated_part)
    } else {
        input
    }
}

/// Lexes the Move source file at the given path and returns a list of completion items
/// corresponding to the non-keyword identifiers therein.
///
/// Currently, this does not perform semantic analysis to determine whether the identifiers
/// returned are valid at the request's cursor position. However, this list of identifiers is akin
/// to what editors like Visual Studio Code would provide as completion items if this language
/// server did not initialize with a response indicating it's capable of providing completions. In
/// the future, the server should be modified to return semantically valid completion items, not
/// simple textual suggestions.
fn lexer_for_buffer(buffer: &str, is_dedup: bool) -> Vec<&str> {
    let mut ids = vec![];
    let mut lexer = Lexer::new(buffer, FileHash::new(buffer));
    if lexer.advance().is_err() {
        return ids;
    }

    while lexer.peek() != Tok::EOF {
        // Some tokens, such as "phantom", are contextual keywords that are only reserved in
        // certain contexts. Since for now this language server doesn't analyze semantic context,
        // tokens such as "phantom" are always present in keyword suggestions. To avoid displaying
        // these keywords to the user twice in the case that the token "phantom" is present in the
        // source program (once as a keyword, and once as an identifier), we filter out any
        // identifier token that has the same text as a keyword.
        if (lexer.peek() == Tok::Identifier && !KEYWORDS.contains(&lexer.content()))
            || lexer.peek() == Tok::ColonColon
            || lexer.peek() == Tok::Period
        {
            // The completion item kind "text" indicates the item is not based on any semantic
            // context of the request cursor's position.
            if !(is_dedup && ids.contains(&lexer.content())) {
                ids.push(lexer.content());
            }
        }
        if lexer.advance().is_err() {
            break;
        }
    }
    ids
}

fn handle_identifiers_for_coloncolon(
    string_tokens: &Vec<&str>,
    env: &GlobalEnv,
) -> Vec<CompletionItem> {
    let mut result = vec![];
    if string_tokens.len() <= 1 {
        return result;
    }
    let prefix_token_string = string_tokens
        .get(string_tokens.len() - 2)
        .unwrap()
        .to_string();
    for module_env in env.get_modules() {
        if prefix_token_string == module_env.get_name().display(env).to_string() {
            for func_env in module_env.get_functions() {
                result.push(completion_item(
                    func_env.get_name_str().as_str(),
                    CompletionItemKind::FUNCTION,
                ));
            }

            for struct_env in module_env.get_structs() {
                result.push(completion_item(
                    &struct_env.get_name().display(env.symbol_pool()).to_string(),
                    CompletionItemKind::STRUCT,
                ));
            }
        }
    }
    result
}

fn handle_identifiers_for_coloncolon_item(
    string_tokens: &Vec<&str>,
    env: &GlobalEnv,
) -> Vec<CompletionItem> {
    let mut result = vec![];
    if string_tokens.len() <= 2 {
        return result;
    }
    let prefix_token_string = string_tokens
        .get(string_tokens.len() - 3)
        .unwrap()
        .to_string();
    let wanted_token_string = string_tokens.last().unwrap();
    for module_env in env.get_modules() {
        if prefix_token_string == module_env.get_name().display(env).to_string() {
            for func_env in module_env.get_functions() {
                if func_env.get_name_str().contains(wanted_token_string) {
                    result.push(completion_item(
                        func_env.get_name_str().as_str(),
                        CompletionItemKind::FUNCTION,
                    ));
                }
            }

            for struct_env in module_env.get_structs() {
                if struct_env
                    .get_name()
                    .display(env.symbol_pool())
                    .to_string()
                    .contains(wanted_token_string)
                {
                    result.push(completion_item(
                        &struct_env.get_name().display(env.symbol_pool()).to_string(),
                        CompletionItemKind::STRUCT,
                    ));
                }
            }
        }
    }
    result
}

fn handle_file_identifiers(
    cursor_tokens: &str,
    file_tokens_string: &mut Vec<&str>,
) -> Vec<CompletionItem> {
    let mut result = vec![];
    file_tokens_string.dedup();
    for &token_str in file_tokens_string.iter() {
        if token_str.contains(cursor_tokens) {
            result.push(completion_item(token_str, CompletionItemKind::TEXT));
        }
    }
    result
}

/// Handles on_completion_request of the language server.
pub fn on_completion_request(context: &Context, request: &Request) -> lsp_server::Response {
    log::info!("on_completion_request request = {:?}", request);
    let parameters = serde_json::from_value::<CompletionParams>(request.params.clone())
        .expect("could not deserialize completion request");
    let fpath = parameters
        .text_document_position
        .text_document
        .uri
        .to_file_path()
        .unwrap();

    let fpath = path_concat(std::env::current_dir().unwrap().as_path(), fpath.as_path());
    let current_project = match context.projects.get_project(&fpath) {
        Some(x) => x,
        None => {
            log::error!("project not found:{:?}", fpath.as_path());
            return Response {
                id: "".to_string().into(),
                result: Some(serde_json::json!({"msg": "No available project"})),
                error: None,
            };
        },
    };

    let file_buffer_str = current_project.current_modifing_file_content.as_str();
    let buffer = Some(file_buffer_str);
    // The completion items we provide depend upon where the user's cursor is positioned.
    let cursor =
        buffer.and_then(|buf| get_cursor_token(buf, &parameters.text_document_position.position));
    let cursor_line =
        buffer.and_then(|buf| get_cursor_line(buf, &parameters.text_document_position.position));

    if cursor_line.is_none() {
        log::error!("could not found code string from cursor line");
        return Response {
            id: "".to_string().into(),
            result: Some(
                serde_json::json!({"msg": "Could not found code string from cursor line"}),
            ),
            error: None,
        };
    }
    let cursor_line_string = cursor_line.unwrap_or("".to_string());
    let truncate_string = truncate_from_last_space(cursor_line_string);
    log::info!("truncate_string: {}", truncate_string);
    let cursor_line_tokens = lexer_for_buffer(&truncate_string, false);

    if cursor_line_tokens.is_empty() {
        log::error!("could not found code string from cursor line");
        return Response {
            id: "".to_string().into(),
            result: Some(
                serde_json::json!({"msg": "Could not found code string from cursor line"}),
            ),
            error: None,
        };
    }
    log::info!("cursor tok: {:?}", cursor);
    let mut items = vec![];
    match cursor {
        Some(Tok::Colon) => {
            items.extend_from_slice(&primitive_types());
        },
        Some(Tok::ColonColon) => {
            // `.` or `::` must be followed by identifiers, which are added to the completion items.
            let module_items =
                handle_identifiers_for_coloncolon(&cursor_line_tokens, &current_project.global_env);
            items.extend_from_slice(&module_items);
        },
        Some(Tok::Period) => {
            log::info!("Tok::Period");
        },
        _ => {
            // If the user's cursor is positioned anywhere other than following a `.`, `:`, or `::`,
            // offer them Move's keywords, operators, and builtins as completion items.
            items.extend_from_slice(&keywords());
            items.extend_from_slice(&builtins());

            let &cursor_token_str = cursor_line_tokens.last().unwrap();
            let mut may_coloncolon = "";
            if cursor_line_tokens.len() > 1 {
                may_coloncolon = cursor_line_tokens
                    .get(cursor_line_tokens.len() - 2)
                    .unwrap();
            }

            if may_coloncolon == "::" {
                let module_items = handle_identifiers_for_coloncolon_item(
                    &cursor_line_tokens,
                    &current_project.global_env,
                );
                items.extend_from_slice(&module_items);
            } else {
                let mut token_str_in_file = lexer_for_buffer(file_buffer_str, true);
                let file_items = handle_file_identifiers(cursor_token_str, &mut token_str_in_file);
                items.extend_from_slice(&file_items);
            }
        },
    }

    // if let Some(cursor_line_buffer) = &cursor_line {
    //     let identifiers = identifiers(cursor_line_buffer.as_str(), &current_project.global_env, &fpath);
    //     // log::info!("identifiers = {:?}", identifiers);
    //     items.extend_from_slice(&identifiers);
    // }

    let result = serde_json::to_value(items).expect("could not serialize completion response");
    let response = lsp_server::Response::new_ok(request.id.clone(), result);
    let ret_response = response.clone();
    log::trace!(
        "------------------------------------\n<on_complete>ret_response = {:?}\n\n",
        ret_response
    );
    if let Err(err) = context
        .connection
        .sender
        .send(lsp_server::Message::Response(response))
    {
        log::error!("could not send completion response: {:?}", err);
    }
    ret_response
}
