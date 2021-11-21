// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use move_cli::package::prover::ProverTest;

#[test]
fn prove_core() {
    ProverTest::create("core").run()
}

#[test]
fn prove_experimental() {
    ProverTest::create("experimental").run()
}

#[test]
fn prove_dpn() {
    ProverTest::create("DPN").run()
}
