// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use move_cli::base::prove::ProverTest;

// TODO: reactivate these tests once the breakage has been resolved.
#[ignore]
#[test]
fn test_diem_framework() {
    ProverTest::create("diem-framework/move-packages/DPN").run()
}
