// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::vm_validator::TransactionValidation;
use anyhow::Result;
use aptos_state_view::StateView;
use aptos_types::{
    account_address::AccountAddress,
    on_chain_config::OnChainConfigPayload,
    transaction::{SignedTransaction, VMValidatorResult},
    vm_status::StatusCode,
};
use aptos_vm::VMValidator;

pub const ACCOUNT_DNE_TEST_ADD: AccountAddress =
    AccountAddress::new([0_u8; AccountAddress::LENGTH]);
pub const INVALID_SIG_TEST_ADD: AccountAddress =
    AccountAddress::new([1_u8; AccountAddress::LENGTH]);
pub const INSUFFICIENT_BALANCE_TEST_ADD: AccountAddress =
    AccountAddress::new([2_u8; AccountAddress::LENGTH]);
pub const SEQ_NUMBER_TOO_NEW_TEST_ADD: AccountAddress =
    AccountAddress::new([3_u8; AccountAddress::LENGTH]);
pub const SEQ_NUMBER_TOO_OLD_TEST_ADD: AccountAddress =
    AccountAddress::new([4_u8; AccountAddress::LENGTH]);
pub const TXN_EXPIRATION_TIME_TEST_ADD: AccountAddress =
    AccountAddress::new([5_u8; AccountAddress::LENGTH]);
pub const INVALID_AUTH_KEY_TEST_ADD: AccountAddress =
    AccountAddress::new([6_u8; AccountAddress::LENGTH]);

#[derive(Clone)]
pub struct MockVMValidator;

impl VMValidator for MockVMValidator {
    fn validate_transaction(
        &self,
        _transaction: SignedTransaction,
        _state_view: &impl StateView,
    ) -> VMValidatorResult {
        VMValidatorResult::new(None, 0)
    }
}

impl TransactionValidation for MockVMValidator {
    type ValidationInstance = MockVMValidator;
    fn validate_transaction(&self, txn: SignedTransaction) -> Result<VMValidatorResult> {
        let txn = match txn.check_signature() {
            Ok(txn) => txn,
            Err(_) => {
                return Ok(VMValidatorResult::new(
                    Some(StatusCode::INVALID_SIGNATURE),
                    0,
                ))
            }
        };

        let sender = txn.sender();
        let ret = if sender == ACCOUNT_DNE_TEST_ADD {
            Some(StatusCode::SENDING_ACCOUNT_DOES_NOT_EXIST)
        } else if sender == INVALID_SIG_TEST_ADD {
            Some(StatusCode::INVALID_SIGNATURE)
        } else if sender == INSUFFICIENT_BALANCE_TEST_ADD {
            Some(StatusCode::INSUFFICIENT_BALANCE_FOR_TRANSACTION_FEE)
        } else if sender == SEQ_NUMBER_TOO_NEW_TEST_ADD {
            Some(StatusCode::SEQUENCE_NUMBER_TOO_NEW)
        } else if sender == SEQ_NUMBER_TOO_OLD_TEST_ADD {
            Some(StatusCode::SEQUENCE_NUMBER_TOO_OLD)
        } else if sender == TXN_EXPIRATION_TIME_TEST_ADD {
            Some(StatusCode::TRANSACTION_EXPIRED)
        } else if sender == INVALID_AUTH_KEY_TEST_ADD {
            Some(StatusCode::INVALID_AUTH_KEY)
        } else {
            None
        };
        Ok(VMValidatorResult::new(ret, 0))
    }

    fn restart(&mut self, _config: OnChainConfigPayload) -> Result<()> {
        unimplemented!();
    }

    fn notify_commit(&mut self) {}
}
