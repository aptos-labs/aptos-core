// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn compile_script_success() {
    let pkg = common::make_package("script_pkg", &[("my_script", "script { fun main() {} }")]);
    let dir = pkg.path().to_str().unwrap();
    let output = common::run_cli(&[
        "compile-script",
        "--package-dir",
        dir,
        "--skip-fetch-latest-git-deps",
    ]);
    // Verify the script.mv file was created
    assert!(
        pkg.path().join("script.mv").exists(),
        "script.mv should be written"
    );
    common::check_baseline(file!(), &output);
}
