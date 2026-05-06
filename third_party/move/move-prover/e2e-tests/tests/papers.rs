// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use std::path::PathBuf;

/// Resolve a path relative to this crate's root (the `e2e-tests` dir).
fn paper_pkg(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

#[test]
fn higher_order_paper_examples() {
    move_prover_e2e_tests::run_paper_with_baseline(paper_pkg(
        "../doc/higher-order-paper-26/examples",
    ));
}
