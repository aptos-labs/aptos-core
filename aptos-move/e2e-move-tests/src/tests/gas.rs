// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::MoveHarness;
use aptos_types::account_config::CORE_CODE_ADDRESS;
use move_core_types::value::MoveValue;
use sha3::{Digest, Sha3_512};

fn dollar_cost(gas_units: u64, price: u64) -> f64 {
    ((gas_units * 100/* gas unit price */) as f64) / 100_000_000_f64 * (price as f64)
}

pub fn print_gas_cost(function: &str, gas_units: u64) {
    println!(
        "{:8} | {:.6} | {:.6} | {:.6} | {}",
        gas_units,
        dollar_cost(gas_units, 5),
        dollar_cost(gas_units, 15),
        dollar_cost(gas_units, 30),
        function,
    );
}

#[test]
fn test_modify_gas_schedule_check_hash() {
    let mut harness = MoveHarness::new();

    let mut gas_schedule = harness.get_gas_schedule();
    let old_hash = Sha3_512::digest(&bcs::to_bytes(&gas_schedule).unwrap()).to_vec();

    const MAGIC: u64 = 42424242;

    let (_, val) = gas_schedule
        .entries
        .iter_mut()
        .find(|(name, _)| name == "instr.nop")
        .unwrap();
    assert_ne!(*val, MAGIC);
    *val = MAGIC;

    harness.executor.exec(
        "gas_schedule",
        "set_for_next_epoch_check_hash",
        vec![],
        vec![
            MoveValue::Signer(CORE_CODE_ADDRESS)
                .simple_serialize()
                .unwrap(),
            bcs::to_bytes(&old_hash).unwrap(),
            bcs::to_bytes(&bcs::to_bytes(&gas_schedule).unwrap()).unwrap(),
        ],
    );

    harness
        .executor
        .exec("reconfiguration_with_dkg", "finish", vec![], vec![
            MoveValue::Signer(CORE_CODE_ADDRESS)
                .simple_serialize()
                .unwrap(),
        ]);

    let (_, gas_params) = harness.get_gas_params();
    assert_eq!(gas_params.vm.instr.nop, MAGIC.into());
}
