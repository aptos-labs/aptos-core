// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::harness::MoveHarness;
use aptos_types::account_address::AccountAddress;
use move_core_types::value::MoveValue;

pub fn initialize_fee_collection_and_distributions(harness: &mut MoveHarness, block_distribution_percentage: u8, batch_distribution_percentage: u8) {
    harness.executor.exec(
        "transaction_fee",
        "initialize_fee_collection_and_distributions",
        vec![],
        vec![
            MoveValue::Signer(AccountAddress::ONE)
                .simple_serialize()
                .unwrap(),
            MoveValue::U8(block_distribution_percentage).simple_serialize().unwrap(),
            MoveValue::U8(batch_distribution_percentage).simple_serialize().unwrap(),
        ],
    );
}

pub fn upgrade_distribution_percentages(harness: &mut MoveHarness, new_block_distribution_percentage: u8, new_batch_distribution_percentage: u8) {
    harness
        .executor
        .exec("transaction_fee", "upgrade_distribution_percentages", vec![], vec![
            MoveValue::Signer(AccountAddress::ONE)
                .simple_serialize()
                .unwrap(),
            MoveValue::U8(new_block_distribution_percentage).simple_serialize().unwrap(),
            MoveValue::U8(new_batch_distribution_percentage).simple_serialize().unwrap(),
        ]);
}
