// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use move_prover::package_prove::ProverTest;

// TODO: split this into individual tests once the package system supports this.

#[test]
fn prove() {
    ProverTest::create(".").run();
    ProverTest::create("nursery").run()
}
