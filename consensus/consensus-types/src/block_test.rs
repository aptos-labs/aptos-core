// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    block::{
        block_test_utils::{certificate_for_genesis, *},
        Block,
    },
    quorum_cert::QuorumCert,
    vote_data::VoteData,
};
use aptos_crypto::{hash::HashValue, test_utils::TestAptosCrypto};
use aptos_types::{
    account_address::AccountAddress,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    on_chain_config::ValidatorSet,
    validator_signer::ValidatorSigner,
    validator_verifier::{random_validator_verifier, ValidatorVerifier},
};
use std::{collections::BTreeMap, sync::Arc};

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

    let nil_block = Block::new_nil(1, quorum_cert);
    assert_eq!(
        nil_block.quorum_cert().certified_block().id(),
        genesis_block.id()
    );
    assert_eq!(nil_block.round(), 1);
    assert_eq!(nil_block.timestamp_usecs(), genesis_block.timestamp_usecs());
    assert_eq!(nil_block.is_nil_block(), true);
    assert!(nil_block.author().is_none());

    let dummy_verifier = Arc::new(ValidatorVerifier::new(BTreeMap::new()));
    assert!(nil_block
        .validate_signature(dummy_verifier.as_ref())
        .is_ok());
    assert!(nil_block.verify_well_formed().is_ok());

    let signer = ValidatorSigner::random(None);
    let payload = vec![];
    let parent_block_info = nil_block.quorum_cert().certified_block();
    let nil_block_qc = gen_test_certificate(
        vec![&signer],
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
        payload,
        2,
        aptos_infallible::duration_since_epoch().as_micros() as u64,
        nil_block_qc,
        &signer,
    );
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
    let payload = vec![];
    let next_block = Block::new_proposal(
        payload.clone(),
        1,
        aptos_infallible::duration_since_epoch().as_micros() as u64,
        quorum_cert,
        &signer,
    );
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
    let signer = ValidatorSigner::random(None);
    let genesis_qc = certificate_for_genesis();
    let round = 1;
    let payload = vec![];
    let current_timestamp = aptos_infallible::duration_since_epoch().as_micros() as u64;
    let block_round_1 = Block::new_proposal(
        payload.clone(),
        round,
        current_timestamp,
        genesis_qc.clone(),
        &signer,
    );

    let signature = signer.sign(genesis_qc.ledger_info().ledger_info());
    let mut ledger_info_altered = genesis_qc.ledger_info().clone();
    ledger_info_altered.add_signature(signer.author(), signature);
    let genesis_qc_altered = QuorumCert::new(genesis_qc.vote_data().clone(), ledger_info_altered);

    let block_round_1_altered = Block::new_proposal(
        payload.clone(),
        round,
        current_timestamp,
        genesis_qc_altered,
        &signer,
    );

    let block_round_1_same =
        Block::new_proposal(payload, round, current_timestamp, genesis_qc, &signer);

    assert!(block_round_1.id() != block_round_1_altered.id());
    assert_eq!(block_round_1.id(), block_round_1_same.id());
}

#[test]
fn test_block_metadata_bitmaps() {
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
    let payload = vec![];
    let start_round = 1;
    let start_timestamp = aptos_infallible::duration_since_epoch().as_micros() as u64;

    let block_1 = Block::new_proposal(
        payload.clone(),
        start_round,
        start_timestamp,
        genesis_qc,
        &signers[0],
    );
    let block_metadata_1 = block_1.new_block_metadata(&validators);
    assert_eq!(signers[0].author(), block_metadata_1.proposer());
    assert_eq!(
        num_validators,
        block_metadata_1.previous_block_votes().len()
    );

    let mut ledger_info_1 = LedgerInfoWithSignatures::new(ledger_info, BTreeMap::new());
    let votes_1 = vec![true, false, true, true];
    votes_1
        .iter()
        .zip(
            validators.iter().zip(
                signers
                    .iter()
                    .map(|signer| signer.sign(&TestAptosCrypto("msg".to_string()))),
            ),
        )
        .for_each(|(&voted, (&address, signature))| {
            if voted {
                ledger_info_1.add_signature(address, signature)
            }
        });
    let qc_1 = QuorumCert::new(
        VoteData::new(BlockInfo::empty(), BlockInfo::empty()),
        ledger_info_1,
    );

    let block_2 = Block::new_proposal(
        payload,
        start_round + 1,
        start_timestamp + 1,
        qc_1,
        &signers[1],
    );
    let block_metadata_2 = block_2.new_block_metadata(&validators);
    assert_eq!(signers[1].author(), block_metadata_2.proposer());
    assert_eq!(&votes_1, block_metadata_2.previous_block_votes());
}

#[test]
fn test_nil_block_metadata_bitmaps() {
    let quorum_cert = certificate_for_genesis();
    let nil_block = Block::new_nil(1, quorum_cert);
    let nil_block_metadata = nil_block.new_block_metadata(&Vec::new());
    assert_eq!(AccountAddress::ZERO, nil_block_metadata.proposer());
    assert_eq!(0, nil_block_metadata.previous_block_votes().len());
}
