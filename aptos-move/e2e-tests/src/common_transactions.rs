// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Support for encoding transactions for common situations.

use crate::account::Account;
use aptos_cached_packages::aptos_stdlib;
use aptos_types::transaction::{Script, SignedTransaction};
use move_ir_compiler::Compiler;
use once_cell::sync::Lazy;
use rand::{rngs::StdRng, SeedableRng};

pub static EMPTY_SCRIPT: Lazy<Vec<u8>> = Lazy::new(|| {
    let code = "
    main(account: signer) {
    label b0:
      return;
    }
";
    let modules = aptos_cached_packages::head_release_bundle().compiled_modules();
    let compiler = Compiler {
        deps: modules.iter().collect(),
    };
    compiler.into_script_blob(code).expect("Failed to compile")
});

// TODO[Orderless]: Need to upgrade these functions to use transaction payload v2 format
pub fn empty_txn(
    sender: &Account,
    seq_num: Option<u64>,
    max_gas_amount: u64,
    gas_unit_price: u64,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> SignedTransaction {
    let seq_num = if use_orderless_transactions {
        u64::MAX
    } else {
        seq_num.unwrap_or(0) // Use 0 as default when seq_num is None
    };
    sender
        .transaction()
        .script(Script::new(EMPTY_SCRIPT.to_vec(), vec![], vec![]))
        .sequence_number(seq_num)
        .max_gas_amount(max_gas_amount)
        .gas_unit_price(gas_unit_price)
        .upgrade_payload(
            &mut rand::thread_rng(),
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
        .sign()
}

/// Returns a transaction to create a new account with the given arguments.
pub fn create_account_txn(
    sender: &Account,
    new_account: &Account,
    seq_num: Option<u64>,
    current_time: u64,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> SignedTransaction {
    let seq_num = if use_orderless_transactions {
        u64::MAX
    } else {
        seq_num.unwrap()
    };
    sender
        .transaction()
        .payload(aptos_stdlib::aptos_account_create_account(
            *new_account.address(),
        ))
        .sequence_number(seq_num)
        .current_time(current_time)
        .upgrade_payload(
            &mut rand::thread_rng(),
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
        .sign()
}

/// Returns a transaction to transfer coin from one account to another (possibly new) one,
/// with the given arguments. Providing 0 as gas_unit_price generates transactions that
/// don't use an aggregator for total supply tracking (due to logic in coin.move that
/// doesn't generate a delta for total supply when gas is 0).
pub fn peer_to_peer_txn(
    sender: &Account,
    receiver: &Account,
    seq_num: Option<u64>,
    transfer_amount: u64,
    gas_unit_price: u64,
    current_time: u64,
    use_txn_payload_v2_format: bool,
    use_orderless_transactions: bool,
) -> SignedTransaction {
    // get a SignedTransaction
    let mut rng: StdRng = SeedableRng::from_seed([0; 32]);
    let seq_num = if use_orderless_transactions {
        u64::MAX
    } else {
        seq_num.unwrap()
    };
    sender
        .transaction()
        .payload(aptos_stdlib::aptos_account_fungible_transfer_only(
            *receiver.address(),
            transfer_amount,
        ))
        .sequence_number(seq_num)
        .current_time(current_time)
        .gas_unit_price(gas_unit_price)
        .upgrade_payload(
            &mut rng,
            use_txn_payload_v2_format,
            use_orderless_transactions,
        )
        .sign()
}
