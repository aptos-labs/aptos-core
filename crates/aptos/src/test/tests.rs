// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::cli_runner::{assert_cmd_not_panic, collect_command_invocations};
use aptos_move_cli::{ArgWithType, FunctionArgType};
use std::str::FromStr;

/// Ensure every visible CLI subcommand parses and does not panic on help / group listings.
///
/// Uses a loop rather than a long chain of `.await` calls to avoid stack overflow in debug tests.
#[tokio::test]
async fn ensure_every_command_args_work() {
    let invocations = collect_command_invocations();
    assert!(
        invocations.len() > 50,
        "expected broad CLI coverage, found only {} invocations",
        invocations.len()
    );

    for args in invocations {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        assert_cmd_not_panic(&arg_refs).await;
    }
}

/// Ensure we can parse URLs for args
#[tokio::test]
async fn ensure_can_parse_args_with_urls() {
    let result = ArgWithType::from_str("string:https://aptoslabs.com").unwrap();
    matches!(result.ty(), FunctionArgType::String);
    assert_eq!(
        result.arg(),
        bcs::to_bytes(&"https://aptoslabs.com".to_string()).unwrap()
    );
}
