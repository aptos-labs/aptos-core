// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::core_mempool::TXN_INDEX_ESTIMATED_BYTES;
use aptos_crypto::HashValue;
use aptos_types::{
    account_address::AccountAddress, account_config::AccountSequenceInfo,
    transaction::SignedTransaction,
};
use serde::{Deserialize, Serialize};
use std::mem::size_of;
use std::time::{Duration, SystemTime};

/// Estimated per-txn size minus the raw transaction
pub const TXN_FIXED_ESTIMATED_BYTES: usize = size_of::<MempoolTransaction>();

#[derive(Clone, Debug)]
pub struct MempoolTransaction {
    pub txn: SignedTransaction,
    // System expiration time of the transaction. It should be removed from mempool by that time.
    pub expiration_time: Duration,
    pub ranking_score: u64,
    pub timeline_state: TimelineState,
    pub sequence_info: SequenceInfo,
    pub insertion_time: SystemTime,
}

impl MempoolTransaction {
    pub(crate) fn new(
        txn: SignedTransaction,
        expiration_time: Duration,
        ranking_score: u64,
        timeline_state: TimelineState,
        seqno_type: AccountSequenceInfo,
        insertion_time: SystemTime,
    ) -> Self {
        Self {
            sequence_info: SequenceInfo {
                transaction_sequence_number: txn.sequence_number(),
                account_sequence_number_type: seqno_type,
            },
            txn,
            expiration_time,
            ranking_score,
            timeline_state,
            insertion_time,
        }
    }
    pub(crate) fn get_sender(&self) -> AccountAddress {
        self.txn.sender()
    }
    pub(crate) fn get_gas_price(&self) -> u64 {
        self.txn.gas_unit_price()
    }
    pub(crate) fn get_committed_hash(&self) -> HashValue {
        self.txn.clone().committed_hash()
    }
    pub(crate) fn get_estimated_bytes(&self) -> usize {
        self.txn.raw_txn_bytes_len() + TXN_FIXED_ESTIMATED_BYTES + TXN_INDEX_ESTIMATED_BYTES
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Hash, Serialize)]
pub enum TimelineState {
    // The transaction is ready for broadcast.
    // Associated integer represents it's position in the log of such transactions.
    Ready(u64),
    // Transaction is not yet ready for broadcast, but it might change in a future.
    NotReady,
    // Transaction will never be qualified for broadcasting.
    // Currently we don't broadcast transactions originated on other peers.
    NonQualified,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct SequenceInfo {
    pub transaction_sequence_number: u64,
    pub account_sequence_number_type: AccountSequenceInfo,
}

#[cfg(test)]
mod test {
    use crate::core_mempool::{MempoolTransaction, TimelineState};
    use aptos_crypto::ed25519::Ed25519PrivateKey;
    use aptos_crypto::PrivateKey;
    use aptos_crypto::SigningKey;
    use aptos_crypto::Uniform;
    use aptos_types::account_address::AccountAddress;
    use aptos_types::account_config::AccountSequenceInfo;
    use aptos_types::chain_id::ChainId;
    use aptos_types::transaction::{RawTransaction, Script, SignedTransaction, TransactionPayload};
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_estimated_bytes() {
        let txn1 = create_test_transaction(0, vec![0x1]);
        let mempool_txn1 = create_test_mempool_transaction(txn1);
        let txn2 = create_test_transaction(0, vec![0x1, 0x2]);
        let mempool_txn2 = create_test_mempool_transaction(txn2);

        assert!(mempool_txn1.get_estimated_bytes() < mempool_txn2.get_estimated_bytes());
    }

    fn create_test_mempool_transaction(signed_txn: SignedTransaction) -> MempoolTransaction {
        MempoolTransaction::new(
            signed_txn,
            Duration::from_secs(1),
            1,
            TimelineState::NotReady,
            AccountSequenceInfo::Sequential(0),
            SystemTime::now(),
        )
    }

    /// Creates a signed transaction
    fn create_test_transaction(sequence_number: u64, code_bytes: Vec<u8>) -> SignedTransaction {
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key = private_key.public_key();

        let transaction_payload =
            TransactionPayload::Script(Script::new(code_bytes, vec![], vec![]));
        let raw_transaction = RawTransaction::new(
            AccountAddress::random(),
            sequence_number,
            transaction_payload,
            0,
            0,
            0,
            ChainId::new(10),
        );
        SignedTransaction::new(
            raw_transaction.clone(),
            public_key,
            private_key.sign(&raw_transaction).unwrap(),
        )
    }
}
