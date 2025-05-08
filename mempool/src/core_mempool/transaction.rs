// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{core_mempool::TXN_INDEX_ESTIMATED_BYTES, counters, network::BroadcastPeerPriority};
use aptos_crypto::HashValue;
use aptos_types::{
    account_address::AccountAddress,
    transaction::{ReplayProtector, SignedTransaction},
};
use serde::{Deserialize, Serialize};
use std::{
    mem::size_of,
    sync::{atomic::AtomicUsize, Arc},
    time::{Duration, SystemTime},
};

/// Estimated per-txn size minus the raw transaction
pub const TXN_FIXED_ESTIMATED_BYTES: usize = size_of::<MempoolTransaction>();

#[derive(Clone, Debug)]
pub struct MempoolTransaction {
    pub txn: SignedTransaction,
    // System expiration time of the transaction. It should be removed from mempool by that time.
    pub expiration_time: Duration,
    pub ranking_score: u64,
    pub timeline_state: TimelineState,
    pub insertion_info: InsertionInfo,
    pub was_parked: bool,
    // The priority of this node for the sender of this transaction.
    pub priority_of_sender: Option<BroadcastPeerPriority>,
}

impl MempoolTransaction {
    pub(crate) fn new(
        txn: SignedTransaction,
        expiration_time: Duration,
        ranking_score: u64,
        timeline_state: TimelineState,
        insertion_time: SystemTime,
        client_submitted: bool,
        priority_of_sender: Option<BroadcastPeerPriority>,
    ) -> Self {
        Self {
            txn,
            expiration_time,
            ranking_score,
            timeline_state,
            insertion_info: InsertionInfo::new(insertion_time, client_submitted, timeline_state),
            was_parked: false,
            priority_of_sender,
        }
    }

    pub(crate) fn get_sender(&self) -> AccountAddress {
        self.txn.sender()
    }

    pub(crate) fn get_replay_protector(&self) -> ReplayProtector {
        self.txn.replay_protector()
    }

    pub(crate) fn get_gas_price(&self) -> u64 {
        self.txn.gas_unit_price()
    }

    pub(crate) fn get_submitted_txn_hash(&self) -> HashValue {
        self.txn.submitted_txn_hash()
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
pub enum SubmittedBy {
    /// The transaction was received from a client REST API submission, rather than a mempool
    /// broadcast. This can be used as the time a transaction first entered the network,
    /// to measure end-to-end latency within the entire network. However, if a transaction is
    /// submitted to multiple nodes (by the client) then the end-to-end latency measured will not
    /// be accurate (the measured value will be lower than the correct value).
    Client,
    /// The transaction was received from a downstream peer, i.e., not a client or a peer validator.
    /// At a validator, a transaction from downstream can be used as the time a transaction first
    /// entered the validator network, to measure end-to-end latency within the validator network.
    /// However, if a transaction enters via multiple validators (due to duplication outside of the
    /// validator network) then the validator end-to-end latency measured will not be accurate
    /// (the measured value will be lower than the correct value).
    Downstream,
    /// The transaction was received at a validator from another validator, rather than from the
    /// downstream VFN. This transaction should not be used to measure end-to-end latency within the
    /// validator network (see Downstream).
    /// Note, with Quorum Store enabled, no transactions will be classified as PeerValidator.
    PeerValidator,
}

#[derive(Debug, Clone)]
pub struct InsertionInfo {
    pub insertion_time: SystemTime,
    pub ready_time: SystemTime,
    pub park_time: Option<SystemTime>,
    pub submitted_by: SubmittedBy,
    pub consensus_pulled_counter: Arc<AtomicUsize>,
}

impl InsertionInfo {
    pub fn new(
        insertion_time: SystemTime,
        client_submitted: bool,
        timeline_state: TimelineState,
    ) -> Self {
        let submitted_by = if client_submitted {
            SubmittedBy::Client
        } else if timeline_state == TimelineState::NonQualified {
            SubmittedBy::PeerValidator
        } else {
            SubmittedBy::Downstream
        };
        Self {
            insertion_time,
            ready_time: insertion_time,
            park_time: None,
            submitted_by,
            consensus_pulled_counter: Arc::new(AtomicUsize::new(0)),
        }
    }

    pub fn submitted_by_label(&self) -> &'static str {
        match self.submitted_by {
            SubmittedBy::Client => counters::SUBMITTED_BY_CLIENT_LABEL,
            SubmittedBy::Downstream => counters::SUBMITTED_BY_DOWNSTREAM_LABEL,
            SubmittedBy::PeerValidator => counters::SUBMITTED_BY_PEER_VALIDATOR_LABEL,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::{
        core_mempool::{MempoolTransaction, TimelineState},
        network::BroadcastPeerPriority,
    };
    use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, SigningKey, Uniform};
    use aptos_types::{
        account_address::AccountAddress,
        chain_id::ChainId,
        transaction::{
            RawTransaction, ReplayProtector, Script, SignedTransaction, TransactionExecutable,
        },
    };
    use std::time::{Duration, SystemTime};

    #[test]
    fn test_estimated_bytes() {
        let txn1 = create_test_transaction(ReplayProtector::SequenceNumber(0), vec![0x1]);
        let mempool_txn1 = create_test_mempool_transaction(txn1);
        let txn2 = create_test_transaction(ReplayProtector::SequenceNumber(0), vec![0x1, 0x2]);
        let mempool_txn2 = create_test_mempool_transaction(txn2);

        assert!(mempool_txn1.get_estimated_bytes() < mempool_txn2.get_estimated_bytes());
    }

    fn create_test_mempool_transaction(signed_txn: SignedTransaction) -> MempoolTransaction {
        MempoolTransaction::new(
            signed_txn,
            Duration::from_secs(1),
            1,
            TimelineState::NotReady,
            SystemTime::now(),
            false,
            Some(BroadcastPeerPriority::Primary),
        )
    }

    /// Creates a signed transaction
    fn create_test_transaction(
        replay_protector: ReplayProtector,
        code_bytes: Vec<u8>,
    ) -> SignedTransaction {
        let private_key = Ed25519PrivateKey::generate_for_testing();
        let public_key = private_key.public_key();

        let transaction_executable =
            TransactionExecutable::Script(Script::new(code_bytes, vec![], vec![]));

        let raw_transaction = RawTransaction::new_txn(
            AccountAddress::random(),
            replay_protector,
            transaction_executable,
            None,
            0,
            0,
            u64::MAX,
            ChainId::new(10),
        );
        SignedTransaction::new(
            raw_transaction.clone(),
            public_key,
            private_key.sign(&raw_transaction).unwrap(),
        )
    }
}
