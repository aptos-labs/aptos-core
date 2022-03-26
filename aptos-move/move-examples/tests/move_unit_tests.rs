// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[test]
fn move_unit_tests() {
    move_cli::package::cli::run_move_unit_tests(
        &std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        move_package::BuildConfig {
            test_mode: true,
            install_dir: Some(tempfile::tempdir().unwrap().path().to_path_buf()),
            ..Default::default()
        },
        move_unit_test::UnitTestingConfig::default_with_bound(Some(100_000)),
        aptos_vm::natives::aptos_natives(),
        /* compute_coverage */ false,
    )
    .unwrap();
}
