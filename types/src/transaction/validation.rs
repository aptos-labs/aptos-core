// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The arguments the transaction prologue and epilogue take, and the abort
//! codes they raise. Both mirror `transaction_validation.move` and are shared
//! by every VM.
//!
//! The argument enums must keep the same BCS serialization: the framework
//! deserializes them directly, so neither the variant order nor the field
//! layout may change. If you need to add a new field, add a new variant here --
//! never edit an existing one.

use crate::{
    fee_statement::FeeStatement,
    transaction::{ReplayProtector, UserTxnLimitsRequest},
};
use move_core_types::account_address::AccountAddress;
use serde::Serialize;

#[derive(Serialize)]
pub enum PrologueArgs {
    V1 {
        needs_fee_payer_auth_check: bool,
        txn_sender_public_key: Option<Vec<u8>>,
        fee_payer_public_key_hash: Option<Vec<u8>>,
        replay_protector: ReplayProtector,
        secondary_signer_addresses: Vec<AccountAddress>,
        secondary_signer_public_key_hashes: Vec<Option<Vec<u8>>>,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        txn_expiration_time: u64,
        chain_id: u8,
        is_simulation: bool,
        txn_limits_request: Option<UserTxnLimitsRequest>,
    },
}

#[derive(Serialize)]
pub enum EpilogueArgs {
    V1 {
        fee_statement: FeeStatement,
        txn_gas_price: u64,
        txn_max_gas_units: u64,
        gas_units_remaining: u64,
        is_simulation: bool,
        is_orderless_txn: bool,
    },
}

/// Abort codes the prologue raises, which the VM translates into specific
/// validation statuses. The prologue must not abort with anything else; doing so
/// is treated as an invariant violation.
//
// TODO(testing): find a way to check these against the Move declarations.
pub const EBAD_ACCOUNT_AUTHENTICATION_KEY: u64 = 1001;
// Transaction sequence number is too old.
pub const ESEQUENCE_NUMBER_TOO_OLD: u64 = 1002;
// Transaction sequence number is too new.
pub const ESEQUENCE_NUMBER_TOO_NEW: u64 = 1003;
// Transaction sender's account does not exist.
pub const EACCOUNT_DOES_NOT_EXIST: u64 = 1004;
// Insufficient balance (to pay for gas deposit).
pub const ECANT_PAY_GAS_DEPOSIT: u64 = 1005;
// Transaction expiration time exceeds block time.
pub const ETRANSACTION_EXPIRED: u64 = 1006;
// chain_id in transaction doesn't match the one on-chain.
pub const EBAD_CHAIN_ID: u64 = 1007;
// Transaction sequence number exceeds u64 max.
pub const ESEQUENCE_NUMBER_TOO_BIG: u64 = 1008;
// Counts of secondary keys and addresses don't match.
pub const ESECONDARY_KEYS_ADDRESSES_COUNT_MISMATCH: u64 = 1009;
// Gas payer account missing in gas payer tx
pub const EGAS_PAYER_ACCOUNT_MISSING: u64 = 1010;
// Insufficient balance to cover the required deposit.
pub const EINSUFFICIENT_BALANCE_FOR_REQUIRED_DEPOSIT: u64 = 1011;
// Nonce is already in the nonce history
pub const ENONCE_ALREADY_USED: u64 = 1012;
// Transaction expiration time is too far in the future.
pub const ETRANSACTION_EXPIRATION_TOO_FAR_IN_FUTURE: u64 = 1013;
