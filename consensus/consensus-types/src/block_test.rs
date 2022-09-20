// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block::{
        block_test_utils::{certificate_for_genesis, *},
        Block,
    },
    common::{Author, Payload},
    quorum_cert::QuorumCert,
    vote_data::VoteData,
};
use aptos_bitvec::BitVec;
use aptos_crypto::hash::HashValue;
use aptos_types::{
    account_address::AccountAddress,
    aggregate_signature::PartialSignatures,
    block_info::{BlockInfo, Round},
    ledger_info::{LedgerInfo, LedgerInfoWithPartialSignatures},
    on_chain_config::ValidatorSet,
    validator_signer::ValidatorSigner,
    validator_verifier::{random_validator_verifier, ValidatorVerifier},
};
use std::sync::Arc;

#[test]
fn test_genesis() {
    // Test genesis and the next block
    let genesis_block = Block::make_genesis_block();
    assert_eq!(genesis_block.parent_id(), HashValue::zero());
    assert_ne!(genesis_block.id(), HashValue::zero());
    assert!(genesis_block.is_genesis_block());
}

#[test]
fn test_nil_block() {
    let genesis_block = Block::make_genesis_block();
    let quorum_cert = certificate_for_genesis();

    let nil_block = Block::new_nil(1, quorum_cert, vec![]);
    assert_eq!(
        nil_block.quorum_cert().certified_block().id(),
        genesis_block.id()
    );
    assert_eq!(nil_block.round(), 1);
    assert_eq!(nil_block.timestamp_usecs(), genesis_block.timestamp_usecs());
    assert_eq!(nil_block.is_nil_block(), true);
    assert!(nil_block.author().is_none());

    let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
    assert!(nil_block
        .validate_signature(dummy_verifier.as_ref())
        .is_ok());
    assert!(nil_block.verify_well_formed().is_ok());

    let signer = ValidatorSigner::random(None);
    let parent_block_info = nil_block.quorum_cert().certified_block();
    let nil_block_qc = gen_test_certificate(
        &[signer.clone()],
        nil_block.gen_block_info(
            parent_block_info.executed_state_id(),
            parent_block_info.version(),
            parent_block_info.next_epoch_state().cloned(),
        ),
        nil_block.quorum_cert().certified_block().clone(),
        None,
    );
    println!(
        "{:?} {:?}",
        nil_block.id(),
        nil_block_qc.certified_block().id()
    );
    let nil_block_child = Block::new_proposal(
        Payload::empty(),
        2,
        aptos_infallible::duration_since_epoch().as_micros() as u64,
        nil_block_qc,
        &signer,
        Vec::new(),
    )
    .unwrap();
    assert_eq!(nil_block_child.is_nil_block(), false);
    assert_eq!(nil_block_child.round(), 2);
    assert_eq!(nil_block_child.parent_id(), nil_block.id());
}

#[test]
fn test_block_relation() {
    let signer = ValidatorSigner::random(None);
    // Test genesis and the next block
    let genesis_block = Block::make_genesis_block();
    let quorum_cert = certificate_for_genesis();
    let payload = Payload::empty();
    let next_block = Block::new_proposal(
        payload.clone(),
        1,
        aptos_infallible::duration_since_epoch().as_micros() as u64,
        quorum_cert,
        &signer,
        Vec::new(),
    )
    .unwrap();
    assert_eq!(next_block.round(), 1);
    assert_eq!(genesis_block.is_parent_of(&next_block), true);
    assert_eq!(
        next_block.quorum_cert().certified_block().id(),
        genesis_block.id()
    );
    assert_eq!(next_block.payload(), Some(&payload));

    let cloned_block = next_block.clone();
    assert_eq!(cloned_block.round(), next_block.round());
}

// Ensure that blocks that extend from the same QuorumCertificate but with different signatures
// have different block ids.
#[test]
fn test_same_qc_different_authors() {
    let (signers, validators) = random_validator_verifier(1, None, false);
    let signer = signers.first().unwrap();
    let genesis_qc = certificate_for_genesis();
    let round = 1;
    let payload = Payload::empty();
    let current_timestamp = aptos_infallible::duration_since_epoch().as_micros() as u64;
    let block_round_1 = Block::new_proposal(
        payload.clone(),
        round,
        current_timestamp,
        genesis_qc.clone(),
        signer,
        Vec::new(),
    )
    .unwrap();

    let signature = signer.sign(genesis_qc.ledger_info().ledger_info()).unwrap();
    let mut ledger_info_altered = LedgerInfoWithPartialSignatures::new(
        genesis_qc.ledger_info().ledger_info().clone(),
        PartialSignatures::empty(),
    );
    ledger_info_altered.add_signature(signer.author(), signature);
    let genesis_qc_altered = QuorumCert::new(
        genesis_qc.vote_data().clone(),
        ledger_info_altered
            .aggregate_signatures(&validators)
            .unwrap(),
    );

    let block_round_1_altered = Block::new_proposal(
        payload.clone(),
        round,
        current_timestamp,
        genesis_qc_altered,
        signer,
        Vec::new(),
    )
    .unwrap();

    let block_round_1_same = Block::new_proposal(
        payload,
        round,
        current_timestamp,
        genesis_qc,
        signer,
        Vec::new(),
    )
    .unwrap();

    assert_ne!(block_round_1.id(), block_round_1_altered.id());
    assert_eq!(block_round_1.id(), block_round_1_same.id());
}

#[test]
fn test_block_metadata_bitvec() {
    let num_validators = 4;
    let (signers, validator_verifier) = random_validator_verifier(num_validators, None, true);
    let validator_set = ValidatorSet::from(&validator_verifier);
    let validators: Vec<_> = validator_verifier
        .get_ordered_account_addresses_iter()
        .collect();
    let ledger_info = LedgerInfo::mock_genesis(Some(validator_set));
    let genesis_qc = QuorumCert::certificate_for_genesis_from_ledger_info(
        &ledger_info,
        Block::make_genesis_block_from_ledger_info(&ledger_info).id(),
    );
    let payload = Payload::empty();
    let start_round = 1;
    let start_timestamp = aptos_infallible::duration_since_epoch().as_micros() as u64;

    let block_1 = Block::new_proposal(
        payload.clone(),
        start_round,
        start_timestamp,
        genesis_qc,
        &signers[0],
        Vec::new(),
    )
    .unwrap();
    let block_metadata_1 = block_1.new_block_metadata(&validators);
    assert_eq!(signers[0].author(), block_metadata_1.proposer());
    assert_eq!(
        BitVec::required_buckets(num_validators as u16),
        block_metadata_1.previous_block_votes_bitvec().len()
    );

    let mut ledger_info_1 =
        LedgerInfoWithPartialSignatures::new(ledger_info.clone(), PartialSignatures::empty());
    let votes_1 = vec![true, false, true, true];
    votes_1
        .iter()
        .zip(
            validators.iter().zip(
                signers
                    .iter()
                    .map(|signer| signer.sign(&ledger_info).unwrap()),
            ),
        )
        .for_each(|(&voted, (&address, signature))| {
            if voted {
                ledger_info_1.add_signature(address, signature)
            }
        });
    let qc_1 = QuorumCert::new(
        VoteData::new(BlockInfo::empty(), BlockInfo::empty()),
        ledger_info_1
            .aggregate_signatures(&validator_verifier)
            .unwrap(),
    );

    let block_2 = Block::new_proposal(
        payload,
        start_round + 1,
        start_timestamp + 1,
        qc_1,
        &signers[1],
        Vec::new(),
    )
    .unwrap();
    let block_metadata_2 = block_2.new_block_metadata(&validators);
    assert_eq!(signers[1].author(), block_metadata_2.proposer());
    let raw_bytes: Vec<u8> = BitVec::from(votes_1).into();
    assert_eq!(&raw_bytes, block_metadata_2.previous_block_votes_bitvec());
}

#[test]
fn test_nil_block_metadata_bitvec() {
    let quorum_cert = certificate_for_genesis();
    let nil_block = Block::new_nil(1, quorum_cert, vec![]);
    let nil_block_metadata = nil_block.new_block_metadata(&Vec::new());
    assert_eq!(AccountAddress::ZERO, nil_block_metadata.proposer());
    assert_eq!(0, nil_block_metadata.previous_block_votes_bitvec().len());
}

#[test]
fn test_failed_authors_well_formed() {
    let signer = ValidatorSigner::random(None);
    let other = Author::random();
    // Test genesis and the next block
    let quorum_cert = certificate_for_genesis();
    let payload = Payload::empty();

    let create_block = |round: Round, failed_authors: Vec<(Round, Author)>| {
        Block::new_proposal(
            payload.clone(),
            round,
            1,
            quorum_cert.clone(),
            &signer,
            failed_authors,
        )
        .unwrap()
    };

    assert!(create_block(1, vec![]).verify_well_formed().is_ok());
    assert!(create_block(2, vec![]).verify_well_formed().is_ok());
    assert!(create_block(2, vec![(1, other)])
        .verify_well_formed()
        .is_ok());
    assert!(create_block(3, vec![(1, other)])
        .verify_well_formed()
        .is_ok());
    assert!(create_block(3, vec![(2, other)])
        .verify_well_formed()
        .is_ok());
    assert!(create_block(3, vec![(1, other), (2, other)])
        .verify_well_formed()
        .is_ok());

    assert!(create_block(1, vec![(0, other)])
        .verify_well_formed()
        .is_err());
    assert!(create_block(2, vec![(0, other)])
        .verify_well_formed()
        .is_err());
    assert!(create_block(2, vec![(2, other)])
        .verify_well_formed()
        .is_err());
    assert!(create_block(2, vec![(1, other), (1, other)])
        .verify_well_formed()
        .is_err());
    assert!(create_block(3, vec![(0, other)])
        .verify_well_formed()
        .is_err());
    assert!(create_block(3, vec![(3, other)])
        .verify_well_formed()
        .is_err());
    assert!(create_block(3, vec![(4, other)])
        .verify_well_formed()
        .is_err());
    assert!(create_block(3, vec![(1, other), (1, other), (1, other)])
        .verify_well_formed()
        .is_err());
    assert!(create_block(3, vec![(1, other), (1, other)])
        .verify_well_formed()
        .is_err());
    assert!(create_block(3, vec![(2, other), (1, other)])
        .verify_well_formed()
        .is_err());
}
