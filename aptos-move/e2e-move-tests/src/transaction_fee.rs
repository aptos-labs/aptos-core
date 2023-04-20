// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::harness::MoveHarness;
use aptos_types::account_address::AccountAddress;
use move_core_types::value::MoveValue;

pub fn initialize_fee_collection_and_distribution(harness: &mut MoveHarness, burn_percentage: u8) {
    harness.executor.exec(
        "transaction_fee",
        "initialize_fee_collection_and_distribution",
        vec![],
        vec![
            MoveValue::Signer(AccountAddress::ONE)
                .simple_serialize()
                .unwrap(),
            MoveValue::U8(burn_percentage).simple_serialize().unwrap(),
        ],
    );
}

pub fn upgrade_burn_percentage(harness: &mut MoveHarness, burn_percentage: u8) {
    harness
        .executor
        .exec("transaction_fee", "upgrade_burn_percentage", vec![], vec![
            MoveValue::Signer(AccountAddress::ONE)
                .simple_serialize()
                .unwrap(),
            MoveValue::U8(burn_percentage).simple_serialize().unwrap(),
        ]);
}
