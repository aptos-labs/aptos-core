// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{hooks::edit, tests::common};

#[test]
fn edit_hook_old_loop_invariant() {
    let source = r#"module 0xCAFE::old_loop {
    fun sum(n: u64): u64 {
        let i = 0;
        let s = 0;
        while (i < n) {
            spec {
                invariant old(i) >= 0;
                invariant old(i + s) >= 0;
            };
            s = s + i;
            i = i + 1;
        };
        s
    }
}
"#;
    let result = edit::check("old_loop_invariant.move", source);
    assert!(!result.has_errors);
    let output = common::sanitize_output(&result.output);
    common::check_baseline(file!(), &output);
}
