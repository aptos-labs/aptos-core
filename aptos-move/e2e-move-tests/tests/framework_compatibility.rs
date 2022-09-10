// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::{ExecutionStatus, TransactionStatus};
use e2e_move_tests::MoveHarness;

mod common;

#[test]
fn can_upgrade_framework() {
    let mut h = MoveHarness::new_testnet();
    h.increase_transaction_size();

    // Upgrade all frameworks in bottom up order as they may have dependencies from each other
    publish(&mut h, "move-stdlib");
    publish(&mut h, "aptos-stdlib");
    publish(&mut h, "aptos-framework");
    publish(&mut h, "aptos-token");
}

fn publish(h: &mut MoveHarness, path: &str) {
    let acc = h.aptos_framework_account();
    match h.publish_package(&acc, &common::framework_dir_path(path)) {
        TransactionStatus::Keep(ExecutionStatus::Success) => {}
        s => {
            panic!("cannot publish `{}`: {:?}", path, s)
        }
    }
}
