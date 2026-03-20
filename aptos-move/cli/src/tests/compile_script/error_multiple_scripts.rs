// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn compile_script_error_multiple_scripts() {
    let pkg = common::make_package("multi_script", &[
        ("script_a", "script { fun main() {} }"),
        ("script_b", "script { fun main() {} }"),
    ]);
    let dir = pkg.path().to_str().unwrap();
    let output = common::run_cli(&[
        "compile-script",
        "--package-dir",
        dir,
        "--skip-fetch-latest-git-deps",
    ]);
    common::check_baseline(file!(), &output);
}
