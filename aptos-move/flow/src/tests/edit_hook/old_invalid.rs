// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{hooks::source_check, tests::common};

#[test]
fn edit_hook_old_invalid() {
    let source = r#"module 0xCAFE::old_invalid {
    fun foo(x: u64): u64 { x + 1 }

    spec foo {
        requires old(x) > 0;
        aborts_if old(x) == 0;
    }
}
"#;
    let result = source_check::check("old_invalid.move", source);
    assert!(result.has_errors);
    assert!(!result.has_parse_errors);
    let output = common::sanitize_output(&result.output);
    common::check_baseline(file!(), &output);
}
