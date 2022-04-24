// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::constants;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    transaction::{RawTransaction, ScriptFunction},
};

/// Builds a `RawTransaction` to handle common transaction values
pub fn build_raw_transaction(
    chain_id: ChainId,
    account: AccountAddress,
    sequence_number: u64,
    script: ScriptFunction,
) -> RawTransaction {
    RawTransaction::new_script_function(
        account,
        sequence_number,
        script,
        constants::MAX_GAS_AMOUNT,
        constants::GAS_UNIT_PRICE,
        TimeService::real().now_secs() + constants::TXN_EXPIRATION_SECS,
        chain_id,
    )
}
