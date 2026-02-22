// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{hooks::edit, tests::common};

#[test]
fn edit_hook_spec_deref_borrow() {
    let source = r#"module 0xCAFE::deref_borrow {
    struct R has key { value: u64 }

    fun get(r: &R): u64 { r.value }

    spec get {
        ensures result == (*r).value;
        ensures result == (&r).value;
    }
}
"#;
    let result = edit::check("spec_deref_borrow.move", source);
    assert!(!result.has_errors);
    let output = common::sanitize_output(&result.output);
    common::check_baseline(file!(), &output);
}
