// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;
use std::os::unix::fs::PermissionsExt;

#[test]
fn fmt_success() {
    let pkg = common::make_package("fmt_pkg", &[(
        "fmt_pkg",
        "module 0xCAFE::fmt_pkg {
    public fun value(): u64 { 42 }
}",
    )]);
    let dir = pkg.path().to_str().unwrap();

    // Create a mock movefmt script that is a no-op (exits 0)
    let mock_bin_dir = pkg.path().join("mock_bin");
    std::fs::create_dir_all(&mock_bin_dir).unwrap();
    let mock_script = mock_bin_dir.join("movefmt");
    std::fs::write(&mock_script, "#!/bin/sh\nexit 0\n").unwrap();
    std::fs::set_permissions(&mock_script, std::fs::Permissions::from_mode(0o755)).unwrap();

    // Point the MOVEFMT_EXE env var to our mock script
    // SAFETY: This test is single-threaded with respect to this env var.
    let mock_script_str = mock_script.to_str().unwrap();
    unsafe {
        std::env::set_var("MOVEFMT_EXE", mock_script_str);
    }

    let output = common::run_cli(&["fmt", "--package-dir", dir]);

    unsafe {
        std::env::remove_var("MOVEFMT_EXE");
    }

    common::check_baseline(file!(), &output);
}
