// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use move_cli::package::prover::ProverTest;

#[test]
fn prove_stdlib() {
    ProverTest::create(".").run()
}

#[test]
fn prove_nursery() {
    ProverTest::create("nursery").run()
}
