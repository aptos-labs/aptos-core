// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    order_vote::OrderVote,
    order_vote_msg::OrderVoteMsg,
    pipeline::{commit_decision::CommitDecision, commit_vote::CommitVote},
    proof_of_store::{BatchInfo, ProofOfStore, SignedBatchInfo},
    quorum_cert::QuorumCert,
    sync_info::SyncInfo,
    vote::Vote,
    vote_data::VoteData,
    vote_msg::VoteMsg,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_types::{
    aggregate_signature::PartialSignatures,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    quorum_store::BatchId,
    validator_verifier::random_validator_verifier,
};
use serde::Serialize;

fn size<T: Serialize>(value: &T) -> usize {
    bcs::to_bytes(value).unwrap().len()
}

/// Prints the serialized size of each consensus message that carries signatures.
/// Run it with `--nocapture`. Run it with and without `aptos-types/pq-consensus`
/// to compare the signature schemes.
#[test]
fn consensus_message_sizes() {
    for num_validators in [50, 106] {
        let (signers, verifier) = random_validator_verifier(num_validators, None, false);
        let quorum = num_validators * 2 / 3 + 1;

        let block = BlockInfo::new(
            1,
            10,
            HashValue::random(),
            HashValue::random(),
            100,
            1,
            None,
        );
        let parent = BlockInfo::new(1, 9, HashValue::random(), HashValue::random(), 90, 0, None);
        let vote_data = VoteData::new(block.clone(), parent);
        let ledger_info = LedgerInfo::new(block, vote_data.hash());

        let mut partial_signatures = PartialSignatures::empty();
        for signer in signers.iter().take(quorum) {
            partial_signatures.add_signature(signer.author(), signer.sign(&ledger_info).unwrap());
        }
        let signatures = verifier
            .aggregate_signatures(partial_signatures.signatures_iter())
            .unwrap();
        let ledger_info_with_signatures =
            LedgerInfoWithSignatures::new(ledger_info.clone(), signatures.clone());
        let quorum_cert = QuorumCert::new(vote_data.clone(), ledger_info_with_signatures.clone());
        quorum_cert.verify(&verifier).unwrap();
        let ordered_cert = quorum_cert.into_wrapped_ledger_info();

        let sync_info = SyncInfo::new_decoupled(
            quorum_cert.clone(),
            ordered_cert.clone(),
            ordered_cert.clone(),
            None,
        );
        let author = signers[0].author();
        let vote = Vote::new(vote_data, author, ledger_info.clone(), &signers[0]).unwrap();
        let vote_msg = VoteMsg::new(vote.clone(), sync_info.clone());
        let order_vote = OrderVote::new_with_signature(
            author,
            ledger_info.clone(),
            signers[0].sign(&ledger_info).unwrap(),
        );
        let order_vote_msg = OrderVoteMsg::new(order_vote, quorum_cert.clone());
        let commit_vote = CommitVote::new(author, ledger_info, &signers[0]).unwrap();
        let commit_decision = CommitDecision::new(ledger_info_with_signatures);

        let batch_info = BatchInfo::new(
            author,
            BatchId::new_for_test(1),
            1,
            1_000_000,
            HashValue::random(),
            100,
            10_000,
            0,
        );
        let signed_batch_info = SignedBatchInfo::new(batch_info.clone(), &signers[0]).unwrap();
        let proof_of_store = ProofOfStore::new(batch_info, signatures);

        println!("--- {num_validators} validators, {quorum} signers ---");
        println!("Vote                 {:>9} B", size(&vote));
        println!("QuorumCert           {:>9} B", size(&quorum_cert));
        println!("SyncInfo             {:>9} B", size(&sync_info));
        println!("VoteMsg              {:>9} B", size(&vote_msg));
        println!("OrderVoteMsg         {:>9} B", size(&order_vote_msg));
        println!("CommitVote           {:>9} B", size(&commit_vote));
        println!("CommitDecision       {:>9} B", size(&commit_decision));
        println!("SignedBatchInfo      {:>9} B", size(&signed_batch_info));
        println!("ProofOfStore         {:>9} B", size(&proof_of_store));
    }
}
