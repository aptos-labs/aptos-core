// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::transaction_metadata::TransactionMetadata;
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{
    block_metadata::BlockMetadata, block_metadata_ext::BlockMetadataExt,
    transaction::ReplayProtector, validator_txn::ValidatorTransaction,
};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

#[derive(BCSCryptoHash, Clone, CryptoHasher, Deserialize, Serialize)]
pub enum SessionId {
    Txn {
        sender: AccountAddress,
        sequence_number: u64,
        script_hash: Vec<u8>,
    },
    BlockMeta {
        // block id
        id: HashValue,
    },
    Genesis {
        // id to identify this specific genesis build
        id: HashValue,
    },
    Prologue {
        sender: AccountAddress,
        sequence_number: u64,
        script_hash: Vec<u8>,
    },
    Epilogue {
        sender: AccountAddress,
        sequence_number: u64,
        script_hash: Vec<u8>,
    },
    // For those runs that are not a transaction and the output of which won't be committed.
    Void,
    RunOnAbort {
        sender: AccountAddress,
        sequence_number: u64,
        script_hash: Vec<u8>,
    },
    BlockMetaExt {
        // block id
        id: HashValue,
    },
    ValidatorTxn {
        script_hash: Vec<u8>,
    },
    OrderlessTxn {
        sender: AccountAddress,
        nonce: u64,
        expiration_time: u64,
        script_hash: Vec<u8>,
    },
    OrderlessTxnProlouge {
        sender: AccountAddress,
        nonce: u64,
        expiration_time: u64,
        script_hash: Vec<u8>,
    },
    OrderlessTxnEpilogue {
        sender: AccountAddress,
        nonce: u64,
        expiration_time: u64,
        script_hash: Vec<u8>,
    },
    OrderlessRunOnAbort {
        sender: AccountAddress,
        nonce: u64,
        expiration_time: u64,
        script_hash: Vec<u8>,
    },
    BlockEpilogue {
        // block id
        id: HashValue,
    },
}

impl SessionId {
    pub fn txn_meta(txn_metadata: &TransactionMetadata) -> Self {
        match txn_metadata.replay_protector() {
            ReplayProtector::SequenceNumber(sequence_number) => Self::Txn {
                sender: txn_metadata.sender,
                sequence_number,
                script_hash: txn_metadata.script_hash.clone(),
            },
            ReplayProtector::Nonce(nonce) => Self::OrderlessTxn {
                sender: txn_metadata.sender,
                nonce,
                expiration_time: txn_metadata.expiration_timestamp_secs,
                script_hash: txn_metadata.script_hash.clone(),
            },
        }
    }

    pub fn genesis(id: HashValue) -> Self {
        Self::Genesis { id }
    }

    pub fn block_meta(block_meta: &BlockMetadata) -> Self {
        Self::BlockMeta {
            id: block_meta.id(),
        }
    }

    pub fn block_meta_ext(block_meta_ext: &BlockMetadataExt) -> Self {
        Self::BlockMetaExt {
            id: block_meta_ext.id(),
        }
    }

    pub fn block_epilogue(id: HashValue) -> Self {
        Self::BlockEpilogue { id }
    }

    pub fn prologue_meta(txn_metadata: &TransactionMetadata) -> Self {
        match txn_metadata.replay_protector() {
            ReplayProtector::SequenceNumber(sequence_number) => Self::Prologue {
                sender: txn_metadata.sender,
                sequence_number,
                script_hash: txn_metadata.script_hash.clone(),
            },
            ReplayProtector::Nonce(nonce) => Self::OrderlessTxnProlouge {
                sender: txn_metadata.sender,
                nonce,
                expiration_time: txn_metadata.expiration_timestamp_secs,
                script_hash: txn_metadata.script_hash.clone(),
            },
        }
    }

    pub fn run_on_abort(txn_metadata: &TransactionMetadata) -> Self {
        match txn_metadata.replay_protector() {
            ReplayProtector::SequenceNumber(sequence_number) => Self::RunOnAbort {
                sender: txn_metadata.sender,
                sequence_number,
                script_hash: txn_metadata.script_hash.clone(),
            },
            ReplayProtector::Nonce(nonce) => Self::OrderlessRunOnAbort {
                sender: txn_metadata.sender,
                nonce,
                expiration_time: txn_metadata.expiration_timestamp_secs,
                script_hash: txn_metadata.script_hash.clone(),
            },
        }
    }

    pub fn epilogue_meta(txn_metadata: &TransactionMetadata) -> Self {
        match txn_metadata.replay_protector() {
            ReplayProtector::SequenceNumber(sequence_number) => Self::Epilogue {
                sender: txn_metadata.sender,
                sequence_number,
                script_hash: txn_metadata.script_hash.clone(),
            },
            ReplayProtector::Nonce(nonce) => Self::OrderlessTxnEpilogue {
                sender: txn_metadata.sender,
                nonce,
                expiration_time: txn_metadata.expiration_timestamp_secs,
                script_hash: txn_metadata.script_hash.clone(),
            },
        }
    }

    pub fn void() -> Self {
        Self::Void
    }

    pub fn validator_txn(txn: &ValidatorTransaction) -> Self {
        Self::ValidatorTxn {
            script_hash: txn.hash().to_vec(),
        }
    }

    pub fn as_uuid(&self) -> HashValue {
        self.hash()
    }

    pub(crate) fn into_script_hash(self) -> Vec<u8> {
        match self {
            Self::Txn { script_hash, .. }
            | Self::Prologue { script_hash, .. }
            | Self::Epilogue { script_hash, .. }
            | Self::RunOnAbort { script_hash, .. }
            | Self::ValidatorTxn { script_hash }
            | Self::OrderlessTxn { script_hash, .. }
            | Self::OrderlessTxnProlouge { script_hash, .. }
            | Self::OrderlessTxnEpilogue { script_hash, .. }
            | Self::OrderlessRunOnAbort { script_hash, .. } => script_hash,
            Self::BlockMeta { id: _ }
            | Self::Genesis { id: _ }
            | Self::Void
            | Self::BlockEpilogue { id: _ }
            | Self::BlockMetaExt { id: _ } => vec![],
        }
    }

    // This is used in `monotonically_increasing_number` native function. Every call to the native function
    // will output a monotonically increasing number.
    // monotonically_increasing_number (128 bits) = 0 (8 bits -- Reserved for future use) || timestamp (64 bits) || transaction_index_inside_block (24 bits) || session_counter_inside_transaction (8 bits) || local_counter_inside_session (16 bits)
    // This function is used to obtain `session_counter_inside_transaction`.
    // The sessions here are organized in the increasing order in which they are created. Eg: Prologue < Txn < RunOnAbort < Epilogue.
    // When introducing new session types, please check the order in which the sessions are created during a transaction execution and assign a number here accordingly.
    pub(crate) fn session_counter(&self) -> u8 {
        match self {
            Self::Genesis { .. } => 0,

            // This session is only used in simulation. Output is not committed.
            // It's okay to use any number here.
            Self::Void => 5,

            // BlockMetadata and BlockMetaData transactions have no sub-sessions.
            // It's okay to use any number here.
            Self::BlockMeta { .. } => 10,
            Self::BlockMetaExt { .. } => 15,

            // Validator transactions have no sub-sessions.
            // It's okay to use any number here.
            Self::ValidatorTxn { .. } => 20,

            // We should maintaint the order: Prologue < Txn < RunOnAbort < Epilogue
            Self::Prologue { .. } => 25,
            Self::OrderlessTxnProlouge { .. } => 30,

            Self::Txn { .. } => 35,
            Self::OrderlessTxn { .. } => 40,

            // RunOnAbort runs before epilogue, so it should be before epilogue
            Self::RunOnAbort { .. } => 45,
            Self::OrderlessRunOnAbort { .. } => 50,

            Self::Epilogue { .. } => 55,
            Self::OrderlessTxnEpilogue { .. } => 60,

            Self::BlockEpilogue { .. } => 65,
        }
    }
}
