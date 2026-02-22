// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{hooks::edit, tests::common};
use std::io::Write;

/// Test that `format_file` actually reformats a deliberately mis-formatted
/// Move source. Skipped when `movefmt` is not installed.
#[test]
fn edit_hook_format() {
    if edit::find_movefmt().is_none() {
        eprintln!("movefmt not found, skipping edit_hook_format");
        return;
    }

    // Deliberately ugly formatting: inconsistent spacing, missing newlines.
    let ugly = r#"module 0xCAFE::fmt_test {    struct S has key {value:u64,}
public fun get(addr:address):u64 {  S[addr].value  }
}
"#;

    let mut tmp = tempfile::Builder::new()
        .suffix(".move")
        .tempfile()
        .expect("create temp file");
    tmp.write_all(ugly.as_bytes()).expect("write temp file");
    let path = tmp.path().to_str().expect("temp path to str").to_owned();

    edit::format_file(&path);

    let after = std::fs::read_to_string(&path).expect("read back formatted file");
    assert_ne!(
        ugly, after,
        "movefmt should have changed the file, but content is identical"
    );

    let output = format!("--- before ---\n{ugly}--- after ---\n{after}");
    common::check_baseline(file!(), &output);
}
