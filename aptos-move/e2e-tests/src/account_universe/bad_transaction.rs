// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_universe::{AUTransactionGen, AccountUniverse},
    common_transactions::{empty_txn, EMPTY_SCRIPT},
    gas_costs,
};
use aptos_crypto::{
    ed25519::{Ed25519PrivateKey, Ed25519PublicKey},
    test_utils::KeyPair,
};
use aptos_gas::{FeePerGasUnit, Gas, InitialGasSchedule, TransactionGasParameters};
use aptos_proptest_helpers::Index;
use aptos_types::{
    transaction::{Script, SignedTransaction, TransactionStatus},
    vm_status::StatusCode,
};
use proptest::prelude::*;
use proptest_derive::Arbitrary;
use std::sync::Arc;

/// Represents a sequence number mismatch transaction
///
#[derive(Arbitrary, Clone, Debug)]
#[proptest(params = "(u64, u64)")]
pub struct SequenceNumberMismatchGen {
    sender: Index,
    #[proptest(strategy = "params.0 ..= params.1")]
    seq: u64,
}

impl AUTransactionGen for SequenceNumberMismatchGen {
    fn apply(
        &self,
        universe: &mut AccountUniverse,
    ) -> (SignedTransaction, (TransactionStatus, u64)) {
        let sender = universe.pick(self.sender).1;

        let seq = if sender.sequence_number == self.seq {
            self.seq + 1
        } else {
            self.seq
        };

        let txn = empty_txn(sender.account(), seq, gas_costs::TXN_RESERVED, 0);

        (
            txn,
            (
                if seq >= sender.sequence_number {
                    TransactionStatus::Discard(StatusCode::SEQUENCE_NUMBER_TOO_NEW)
                } else {
                    TransactionStatus::Discard(StatusCode::SEQUENCE_NUMBER_TOO_OLD)
                },
                0,
            ),
        )
    }
}

/// Represents a insufficient balance transaction
///
#[derive(Arbitrary, Clone, Debug)]
#[proptest(params = "(u64, u64)")]
pub struct InsufficientBalanceGen {
    sender: Index,
    #[proptest(strategy = "params.0 ..= params.1")]
    gas_unit_price: u64,
}

impl AUTransactionGen for InsufficientBalanceGen {
    fn apply(
        &self,
        universe: &mut AccountUniverse,
    ) -> (SignedTransaction, (TransactionStatus, u64)) {
        let sender = universe.pick(self.sender).1;

        let max_gas_unit = (sender.balance / self.gas_unit_price) + 1;

        let txn = empty_txn(
            sender.account(),
            sender.sequence_number,
            max_gas_unit,
            self.gas_unit_price,
        );

        // TODO: Move such config to AccountUniverse
        let txn_gas_params = TransactionGasParameters::initial();
        let raw_bytes_len = txn.raw_txn_bytes_len() as u64;
        let min_cost: Gas = txn_gas_params
            .calculate_intrinsic_gas(raw_bytes_len.into())
            .to_unit_round_up_with_params(&txn_gas_params);

        (
            txn,
            (
                if Gas::from(max_gas_unit) > txn_gas_params.maximum_number_of_gas_units {
                    TransactionStatus::Discard(
                        StatusCode::MAX_GAS_UNITS_EXCEEDS_MAX_GAS_UNITS_BOUND,
                    )
                } else if Gas::from(max_gas_unit) < min_cost {
                    TransactionStatus::Discard(
                        StatusCode::MAX_GAS_UNITS_BELOW_MIN_TRANSACTION_GAS_UNITS,
                    )
                } else if FeePerGasUnit::from(self.gas_unit_price)
                    > txn_gas_params.max_price_per_gas_unit
                {
                    TransactionStatus::Discard(StatusCode::GAS_UNIT_PRICE_ABOVE_MAX_BOUND)
                } else if FeePerGasUnit::from(self.gas_unit_price)
                    < txn_gas_params.min_price_per_gas_unit
                {
                    TransactionStatus::Discard(StatusCode::GAS_UNIT_PRICE_BELOW_MIN_BOUND)
                } else {
                    TransactionStatus::Discard(StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE)
                },
                0,
            ),
        )
    }
}

/// Represents a authkey mismatch transaction
///
#[derive(Arbitrary, Clone, Debug)]
#[proptest(no_params)]
pub struct InvalidAuthkeyGen {
    sender: Index,
    #[proptest(
        strategy = "aptos_crypto::test_utils::uniform_keypair_strategy_with_perturbation(1)"
    )]
    new_keypair: KeyPair<Ed25519PrivateKey, Ed25519PublicKey>,
}

impl AUTransactionGen for InvalidAuthkeyGen {
    fn apply(
        &self,
        universe: &mut AccountUniverse,
    ) -> (SignedTransaction, (TransactionStatus, u64)) {
        let sender = universe.pick(self.sender).1;

        let txn = sender
            .account()
            .transaction()
            .script(Script::new(EMPTY_SCRIPT.clone(), vec![], vec![]))
            .sequence_number(sender.sequence_number)
            .raw()
            .sign(
                &self.new_keypair.private_key,
                self.new_keypair.public_key.clone(),
            )
            .unwrap()
            .into_inner();

        (
            txn,
            (TransactionStatus::Discard(StatusCode::INVALID_AUTH_KEY), 0),
        )
    }
}

pub fn bad_txn_strategy() -> impl Strategy<Value = Arc<dyn AUTransactionGen + 'static>> {
    prop_oneof![
        1 => any_with::<SequenceNumberMismatchGen>((0, 10_000)).prop_map(SequenceNumberMismatchGen::arced),
        1 => any_with::<InvalidAuthkeyGen>(()).prop_map(InvalidAuthkeyGen::arced),
        1 => any_with::<InsufficientBalanceGen>((1, 20_000)).prop_map(InsufficientBalanceGen::arced),
    ]
}
