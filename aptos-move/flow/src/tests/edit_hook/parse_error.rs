// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{hooks::edit, tests::common};

#[test]
fn edit_hook_parse_error() {
    let source = r#"module 0xCAFE::broken {
    fun foo(): u64 { true
"#;
    let result = edit::check("parse_error.move", source);
    assert!(result.has_errors);
    let output = common::sanitize_output(&result.output);
    common::check_baseline(file!(), &output);
}
