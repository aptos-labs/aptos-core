// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_shuffler::deprecated_fairness::conflict_key::ConflictKey;
use aptos_types::transaction::SignedTransaction;
use move_core_types::account_address::AccountAddress;

#[derive(Eq, Hash, PartialEq)]
pub struct TxnSenderKey(AccountAddress);

impl ConflictKey<SignedTransaction> for TxnSenderKey {
    fn extract_from(txn: &SignedTransaction) -> Self {
        TxnSenderKey(txn.sender())
    }

    fn conflict_exempt(&self) -> bool {
        false
    }
}
