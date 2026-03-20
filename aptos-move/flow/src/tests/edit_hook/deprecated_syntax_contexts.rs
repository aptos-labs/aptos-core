// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::hooks::source_check;

#[test]
fn edit_hook_deprecated_syntax_ignores_comments_and_strings() {
    let source = r#"module 0xCAFE::deprecated_syntax_contexts {
    struct Token has key {}

    /*
        borrow_global<Token>(@0xCAFE);
        borrow_global_mut<Token>(@0xCAFE);
        acquires Token
    */
    fun context_only() {
        let s1 = b"borrow_global<Token>(@0xCAFE)";
        let s2 = b"borrow_global_mut<Token>(@0xCAFE)";
        let s3 = b"text acquires Token";
        // borrow_global<Token>(@0xCAFE)
        // borrow_global_mut<Token>(@0xCAFE)
        // acquires Token
        let _ = (s1, s2, s3);
    }
}
"#;

    let result = source_check::check("deprecated_syntax_contexts.move", source);
    assert!(
        !result.has_errors,
        "unexpected diagnostics:\n{}",
        result.output
    );
    assert!(!result.has_parse_errors);
    assert!(
        result.output.is_empty(),
        "unexpected output:\n{}",
        result.output
    );
}
