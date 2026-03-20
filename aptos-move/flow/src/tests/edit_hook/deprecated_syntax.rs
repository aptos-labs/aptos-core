// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{hooks::source_check, tests::common};

#[test]
fn edit_hook_deprecated_syntax() {
    let source = r#"module 0xCAFE::old_style {
    struct Token has key {
        value: u64,
    }

    fun get(addr: address): &Token acquires Token {
        borrow_global<Token>(addr)
    }

    fun get_mut(addr: address): &mut Token acquires Token {
        borrow_global_mut<Token>(addr)
    }

    fun remove(addr: address): Token acquires Token {
        move_from<Token>(addr)
    }

    fun store(s: &signer, token: Token) {
        move_to(s, token)
    }
}
"#;
    let result = source_check::check("deprecated_syntax.move", source);
    assert!(result.has_errors);
    assert!(!result.has_parse_errors);
    let output = common::sanitize_output(&result.output);
    common::check_baseline(file!(), &output);
}
