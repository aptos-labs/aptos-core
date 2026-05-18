// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

#[test]
fn document_success() {
    let pkg = common::make_package("greeter", &[(
        "greeter",
        "/// A greeting module
module 0xCAFE::greeter {
    /// Returns 42
    public fun greet(): u64 { 42 }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();
    let output = common::run_cli(&[
        "document",
        "--package-dir",
        dir,
        "--skip-fetch-latest-git-deps",
    ]);
    common::check_baseline(file!(), &output);

    // Verify doc directory was created
    assert!(
        pkg.path().join("doc").exists(),
        "doc/ directory should be created"
    );
}
