// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    contract_event::ContractEvent,
    state_store::state_key::StateKey,
    timestamp::TimestampResource,
    transaction::{
        BlockEndInfo, BlockExecutableTransaction, FeeDistribution, SignedTransaction,
        TBlockEndInfoExt, Transaction,
    },
    write_set::WriteOp,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum SignatureVerifiedTransaction {
    Valid(Transaction),
    Invalid(Transaction),
}

impl PartialEq for SignatureVerifiedTransaction {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                SignatureVerifiedTransaction::Invalid(a),
                SignatureVerifiedTransaction::Invalid(b),
            ) => a.eq(b),
            (SignatureVerifiedTransaction::Valid(a), SignatureVerifiedTransaction::Valid(b)) => {
                a.eq(b)
            },
            _ => {
                panic!("Unexpected equality check on {:?} and {:?}", self, other)
            },
        }
    }
}

impl Eq for SignatureVerifiedTransaction {}

impl SignatureVerifiedTransaction {
    pub fn into_inner(self) -> Transaction {
        match self {
            SignatureVerifiedTransaction::Valid(txn) => txn,
            SignatureVerifiedTransaction::Invalid(txn) => txn,
        }
    }

    pub fn borrow_into_inner(&self) -> &Transaction {
        match self {
            SignatureVerifiedTransaction::Valid(txn) => txn,
            SignatureVerifiedTransaction::Invalid(txn) => txn,
        }
    }

    pub fn is_valid(&self) -> bool {
        match self {
            SignatureVerifiedTransaction::Valid(_) => true,
            SignatureVerifiedTransaction::Invalid(_) => false,
        }
    }

    pub fn sender(&self) -> Option<AccountAddress> {
        match self {
            SignatureVerifiedTransaction::Valid(txn) => match txn {
                Transaction::UserTransaction(txn) => Some(txn.sender()),
                _ => None,
            },
            SignatureVerifiedTransaction::Invalid(_) => None,
        }
    }

    pub fn hash(&self) -> HashValue {
        match self {
            SignatureVerifiedTransaction::Valid(txn) => txn.hash(),
            SignatureVerifiedTransaction::Invalid(txn) => txn.hash(),
        }
    }

    pub fn expect_valid(&self) -> &Transaction {
        match self {
            SignatureVerifiedTransaction::Valid(txn) => txn,
            SignatureVerifiedTransaction::Invalid(_) => panic!("Expected valid transaction"),
        }
    }
}

impl BlockExecutableTransaction for SignatureVerifiedTransaction {
    type Event = ContractEvent;
    type Key = StateKey;
    type Tag = StructTag;
    type Value = WriteOp;

    fn user_txn_bytes_len(&self) -> usize {
        match self {
            SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(txn)) => {
                txn.txn_bytes_len()
            },
            _ => 0,
        }
    }

    fn try_as_signed_user_txn(&self) -> Option<&SignedTransaction> {
        match self {
            SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(txn)) => Some(txn),
            _ => None,
        }
    }

    fn state_checkpoint(block_id: HashValue) -> Self {
        Transaction::StateCheckpoint(block_id).into()
    }

    fn block_epilogue_v0(block_id: HashValue, block_end_info: BlockEndInfo) -> Self {
        Transaction::block_epilogue_v0(block_id, block_end_info).into()
    }

    fn block_epilogue_v1(
        block_id: HashValue,
        block_end_info: TBlockEndInfoExt<Self::Key>,
        fee_distribution: FeeDistribution,
    ) -> Self {
        Transaction::block_epilogue_v1(block_id, block_end_info, fee_distribution).into()
    }

    fn pre_write_values(&self) -> Vec<(Self::Key, Self::Value)> {
        let timestamp = match self {
            SignatureVerifiedTransaction::Valid(Transaction::BlockMetadataExt(metadata_txn)) => {
                Some(metadata_txn.timestamp_usecs())
            },
            SignatureVerifiedTransaction::Valid(Transaction::BlockMetadata(metadata_txn)) => {
                Some(metadata_txn.timestamp_usecs())
            },
            _ => None,
        };

        match timestamp {
            Some(ts) => {
                // Use typed StateKey creation to avoid string parsing.
                // These unwraps are safe: TimestampResource is a valid MoveResource type,
                // and u64 serialization via BCS cannot fail.
                let state_key =
                    StateKey::resource_typed::<TimestampResource>(&AccountAddress::ONE)
                        .expect("TimestampResource is a valid MoveResource");
                let value = WriteOp::legacy_modification(
                    bcs::to_bytes(&ts).expect("u64 BCS serialization cannot fail").into(),
                );
                vec![(state_key, value)]
            },
            None => vec![],
        }
    }
}

impl From<Transaction> for SignatureVerifiedTransaction {
    fn from(txn: Transaction) -> Self {
        match txn {
            Transaction::UserTransaction(txn) => match txn.verify_signature() {
                Ok(_) => SignatureVerifiedTransaction::Valid(Transaction::UserTransaction(txn)),
                Err(_) => SignatureVerifiedTransaction::Invalid(Transaction::UserTransaction(txn)),
            },
            _ => SignatureVerifiedTransaction::Valid(txn),
        }
    }
}

pub fn into_signature_verified_block(txns: Vec<Transaction>) -> Vec<SignatureVerifiedTransaction> {
    txns.into_iter().map(|t| t.into()).collect()
}

pub trait TransactionProvider: Debug {
    fn get_transaction(&self) -> Option<&Transaction>;
}

impl TransactionProvider for SignatureVerifiedTransaction {
    fn get_transaction(&self) -> Option<&Transaction> {
        if self.is_valid() {
            Some(self.expect_valid())
        } else {
            None
        }
    }
}

impl TransactionProvider for Transaction {
    fn get_transaction(&self) -> Option<&Transaction> {
        Some(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_metadata::BlockMetadata;
    use aptos_crypto::HashValue;

    #[test]
    fn test_pre_write_values_for_block_metadata() {
        let timestamp_usecs = 1234567890u64;
        let block_metadata = BlockMetadata::new(
            HashValue::zero(),
            1,  // epoch
            1,  // round
            AccountAddress::ONE,
            vec![],  // previous_block_votes_bitvec
            vec![],  // failed_proposer_indices
            timestamp_usecs,
        );

        let txn = SignatureVerifiedTransaction::Valid(Transaction::BlockMetadata(block_metadata));
        let pre_write_values = txn.pre_write_values();

        // Should return exactly one pre-write entry for the timestamp
        assert_eq!(pre_write_values.len(), 1);

        let (state_key, write_op) = &pre_write_values[0];

        // Verify the state key is for the timestamp resource
        let expected_state_key =
            StateKey::resource_typed::<TimestampResource>(&AccountAddress::ONE)
                .expect("TimestampResource is a valid MoveResource");
        assert_eq!(state_key, &expected_state_key);

        // Verify the value is the serialized timestamp
        let expected_value = bcs::to_bytes(&timestamp_usecs).unwrap();
        assert_eq!(write_op.bytes(), Some(&expected_value.into()));
    }

    #[test]
    fn test_pre_write_values_for_user_transaction_returns_empty() {
        // For non-block-metadata transactions, pre_write_values should return empty
        let state_checkpoint_txn =
            SignatureVerifiedTransaction::Valid(Transaction::StateCheckpoint(HashValue::zero()));
        assert!(state_checkpoint_txn.pre_write_values().is_empty());
    }
}
