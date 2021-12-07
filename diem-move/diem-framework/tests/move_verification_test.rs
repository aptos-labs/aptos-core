// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use move_cli::package::prover::ProverTest;

// TODO: split this into individual tests once the package system supports this.
#[test]
fn prove_all() {
    ProverTest::create("core").run();
    ProverTest::create("experimental").run();
    ProverTest::create("DPN").run()
}
