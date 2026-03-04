// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{hooks::source_check, tests::common};

#[test]
fn edit_hook_clean() {
    let source = r#"
module 0xCAFE::clean {
    struct Counter has key {
        value: u64,
    }

    public fun get(addr: address): u64 {
        Counter[addr].value
    }

    spec get {
        ensures result == Counter[addr].value;
        ensures old(Counter[addr].value) == result;
    }
}
"#;
    let result = source_check::check("clean.move", source);
    assert!(!result.has_errors);
    assert!(!result.has_parse_errors);
    let output = common::sanitize_output(&result.output);
    common::check_baseline(file!(), &output);
}
