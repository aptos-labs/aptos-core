// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::tests::common;

/// `--checks=<typo>` must fail before package resolution. We point at a
/// non-existent `--package-dir` so that, if validation slipped past spec
/// checking, the error would mention the missing path. The fact that the
/// error mentions the unknown lint name (and not `Move.toml` / package
/// resolution) proves the spec is validated up front, before any code path
/// that could fetch git deps or mutate Move.lock runs.
#[test]
fn unknown_lint_name_errors_before_package_resolution() {
    let output = common::run_cli(&[
        "lint",
        "--package-dir",
        "/definitely/does/not/exist/aptos-lint-typo",
        "--skip-fetch-latest-git-deps",
        "--checks=not_a_real_lint",
    ]);
    let err = format!("{:?}", output.result);
    assert!(
        err.contains("not_a_real_lint"),
        "expected `unknown lint check` error mentioning the typo; got: {err}\nstderr: {}",
        output.stderr
    );
    assert!(
        !err.contains("Move.toml") && !output.stderr.contains("Move.toml"),
        "validation must run before package resolution; \
         got error mentioning Move.toml.\nerr: {err}\nstderr: {}",
        output.stderr
    );
}
