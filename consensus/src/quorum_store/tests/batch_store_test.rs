// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    network_interface::ConsensusMsg,
    quorum_store::{
        batch_reader::{BatchReader, BatchReaderCommand},
        batch_store::{BatchStore, BatchStoreCommand, PersistRequest},
        proof_coordinator::ProofCoordinatorCommand,
        quorum_store_db::QuorumStoreDB,
        tests::utils::{compute_digest_from_signed_transaction, create_vec_signed_transactions},
        types::SerializedTransaction,
    },
    test_utils::mock_quorum_store_sender::MockQuorumStoreSender,
};
use aptos_config::config::QuorumStoreConfig;
use aptos_consensus_types::{common::Author, proof_of_store::LogicalTime};
use aptos_crypto::HashValue;
use aptos_logger::spawn_named;
use aptos_temppath::TempPath;
use aptos_types::{
    transaction::SignedTransaction,
    validator_signer::ValidatorSigner,
    validator_verifier::{random_validator_verifier, ValidatorVerifier},
    PeerId,
};
use std::sync::Arc;

struct TestBatchStore<T> {
    pub batch_store: BatchStore<T>,
    pub _batch_reader: Arc<BatchReader>,
    pub network_rx: tokio::sync::mpsc::Receiver<(ConsensusMsg, Vec<Author>)>,
    pub batch_store_cmd_tx: tokio::sync::mpsc::Sender<BatchStoreCommand>,
    pub batch_store_cmd_rx: tokio::sync::mpsc::Receiver<BatchStoreCommand>,
    pub batch_reader_cmd_tx: tokio::sync::mpsc::Sender<BatchReaderCommand>,
}

fn start_batch_store(
    signer: ValidatorSigner,
    db_path: &TempPath,
    validator_verifier: &ValidatorVerifier,
) -> TestBatchStore<MockQuorumStoreSender> {
    let config = QuorumStoreConfig::default();
    let (network_tx, network_rx) = tokio::sync::mpsc::channel(100);
    let network_sender = MockQuorumStoreSender::new(network_tx);

    let (batch_store_cmd_tx, batch_store_cmd_rx) = tokio::sync::mpsc::channel(100);
    let (batch_reader_cmd_tx, batch_reader_cmd_rx) = tokio::sync::mpsc::channel(100);
    let (batch_store, _batch_reader) = BatchStore::new(
        0,
        0,
        signer.author(),
        network_sender,
        batch_store_cmd_tx.clone(),
        batch_reader_cmd_tx.clone(),
        batch_reader_cmd_rx,
        Arc::new(QuorumStoreDB::new(db_path)),
        validator_verifier.clone(),
        Arc::new(signer),
        config.batch_expiry_round_gap_when_init,
        config.batch_expiry_round_gap_behind_latest_certified,
        config.batch_expiry_round_gap_beyond_latest_certified,
        config.batch_expiry_grace_rounds,
        config.batch_request_num_peers,
        config.batch_request_timeout_ms,
        config.memory_quota,
        config.db_quota,
    );
    TestBatchStore {
        batch_store,
        _batch_reader,
        network_rx,
        batch_store_cmd_tx,
        batch_store_cmd_rx,
        batch_reader_cmd_tx,
    }
}

async fn get_batch_for_peer_and_check(
    network_rx: &mut tokio::sync::mpsc::Receiver<(ConsensusMsg, Vec<Author>)>,
    batch_reader_cmd_tx: tokio::sync::mpsc::Sender<BatchReaderCommand>,
    digest_hash: HashValue,
    remote_peer_id: PeerId,
    expected_txns: &[SignedTransaction],
) {
    let cmd = BatchReaderCommand::GetBatchForPeer(digest_hash, remote_peer_id);
    batch_reader_cmd_tx.send(cmd).await.expect("Could not send");
    let (msg, peer_ids) = network_rx.recv().await.expect("Could not recv");
    assert_eq!(peer_ids.len(), 1);
    assert_eq!(peer_ids[0], remote_peer_id);
    match msg {
        ConsensusMsg::BatchMsg(batch) => {
            let txns = batch.into_payload();
            assert_eq!(txns.len(), expected_txns.len());
            for (txn, expected_txn) in txns.iter().zip(expected_txns) {
                assert_eq!(txn, expected_txn);
            }
        },
        _ => panic!("Unexpected msg {:?}", msg),
    }
}

async fn shutdown(batch_store_cmd_tx: tokio::sync::mpsc::Sender<BatchStoreCommand>) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let cmd = BatchStoreCommand::Shutdown(tx);
    batch_store_cmd_tx.send(cmd).await.expect("Could not send");
    rx.await.expect("Could not shutdown");
}

#[ignore] // TODO: debug and re-enable before deploying quorum store
#[tokio::test(flavor = "multi_thread")]
async fn test_batch_store_recovery() {
    let tmp_dir = TempPath::new();
    let txns = create_vec_signed_transactions(100);
    let digest_hash = compute_digest_from_signed_transaction(txns.clone());
    let num_bytes = txns
        .iter()
        .map(SerializedTransaction::from_signed_txn)
        .map(|t| t.len())
        .sum();
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let peer_id = signers[0].author();
    let remote_peer_id = signers[1].author();

    {
        let test_batch_store = start_batch_store(signers[0].clone(), &tmp_dir, &validator_verifier);
        let (proof_coordinator_tx, mut proof_coordinator_rx) = tokio::sync::mpsc::channel(100);
        spawn_named!(
            "batch store",
            test_batch_store
                .batch_store
                .start(test_batch_store.batch_store_cmd_rx, proof_coordinator_tx)
        );

        // Persist batch and wait
        let cmd = BatchStoreCommand::Persist(PersistRequest::new(
            peer_id,
            txns.clone(),
            digest_hash,
            num_bytes,
            LogicalTime::new(0, 100),
        ));
        test_batch_store
            .batch_store_cmd_tx
            .clone()
            .send(cmd)
            .await
            .expect("Could not send");
        let msg = proof_coordinator_rx.recv().await.expect("Could not recv");
        match msg {
            ProofCoordinatorCommand::AppendSignature(digest) => {
                assert_eq!(digest.digest(), digest_hash);
            },
            _ => panic!("Unexpected msg {:?}", msg),
        }

        let mut network_rx = test_batch_store.network_rx;
        get_batch_for_peer_and_check(
            &mut network_rx,
            test_batch_store.batch_reader_cmd_tx.clone(),
            digest_hash,
            remote_peer_id,
            &txns,
        )
        .await;
        shutdown(test_batch_store.batch_store_cmd_tx.clone()).await;
    }

    {
        let test_batch_store = start_batch_store(signers[0].clone(), &tmp_dir, &validator_verifier);
        let (proof_coordinator_tx, _proof_coordinator_rx) = tokio::sync::mpsc::channel(100);
        spawn_named!(
            "batch store restart",
            test_batch_store
                .batch_store
                .start(test_batch_store.batch_store_cmd_rx, proof_coordinator_tx)
        );

        let mut network_rx = test_batch_store.network_rx;
        get_batch_for_peer_and_check(
            &mut network_rx,
            test_batch_store.batch_reader_cmd_tx.clone(),
            digest_hash,
            remote_peer_id,
            &txns,
        )
        .await;
        shutdown(test_batch_store.batch_store_cmd_tx.clone()).await;
    }
}
