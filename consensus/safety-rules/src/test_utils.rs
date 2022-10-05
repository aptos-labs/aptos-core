// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    persistent_safety_storage::PersistentSafetyStorage, serializer::SerializerService, SafetyRules,
    TSafetyRules,
};
use aptos_crypto::hash::{CryptoHash, TransactionAccumulatorHasher};
use aptos_secure_storage::{InMemoryStorage, Storage};
use aptos_types::{
    aggregate_signature::{AggregateSignature, PartialSignatures},
    block_info::BlockInfo,
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    ledger_info::{LedgerInfo, LedgerInfoWithPartialSignatures, LedgerInfoWithSignatures},
    on_chain_config::ValidatorSet,
    proof::AccumulatorExtensionProof,
    validator_info::ValidatorInfo,
    validator_signer::ValidatorSigner,
    validator_verifier::generate_validator_verifier,
    waypoint::Waypoint,
};
use consensus_types::timeout_2chain::TwoChainTimeoutWithPartialSignatures;
use consensus_types::{
    block::Block,
    common::{Payload, Round},
    quorum_cert::QuorumCert,
    timeout_2chain::{TwoChainTimeout, TwoChainTimeoutCertificate},
    vote::Vote,
    vote_data::VoteData,
    vote_proposal::VoteProposal,
};

pub type Proof = AccumulatorExtensionProof<TransactionAccumulatorHasher>;

pub fn empty_proof() -> Proof {
    Proof::new(vec![], 0, vec![])
}

pub fn make_genesis(signer: &ValidatorSigner) -> (EpochChangeProof, QuorumCert) {
    let validator_info =
        ValidatorInfo::new_with_test_network_keys(signer.author(), signer.public_key(), 1, 0);
    let validator_set = ValidatorSet::new(vec![validator_info]);
    let li = LedgerInfo::mock_genesis(Some(validator_set));
    let block = Block::make_genesis_block_from_ledger_info(&li);
    let qc = QuorumCert::certificate_for_genesis_from_ledger_info(&li, block.id());
    let lis = LedgerInfoWithSignatures::new(li, AggregateSignature::empty());
    let proof = EpochChangeProof::new(vec![lis], false);
    (proof, qc)
}

pub fn make_proposal_with_qc_and_proof(
    payload: Payload,
    round: Round,
    proof: Proof,
    qc: QuorumCert,
    validator_signer: &ValidatorSigner,
) -> VoteProposal {
    VoteProposal::new(
        proof,
        Block::new_proposal(
            payload,
            round,
            qc.certified_block().timestamp_usecs() + 1,
            qc,
            validator_signer,
            Vec::new(),
        )
        .unwrap(),
        None,
        false,
    )
}

pub fn make_proposal_with_qc(
    round: Round,
    qc: QuorumCert,
    validator_signer: &ValidatorSigner,
) -> VoteProposal {
    make_proposal_with_qc_and_proof(Payload::empty(), round, empty_proof(), qc, validator_signer)
}

pub fn make_proposal_with_parent_and_overrides(
    payload: Payload,
    round: Round,
    parent: &VoteProposal,
    committed: Option<&VoteProposal>,
    validator_signer: &ValidatorSigner,
    epoch: Option<u64>,
    next_epoch_state: Option<EpochState>,
) -> VoteProposal {
    let block_epoch = match epoch {
        Some(e) => e,
        _ => parent.block().epoch(),
    };

    let parent_output = parent
        .accumulator_extension_proof()
        .verify(
            parent
                .block()
                .quorum_cert()
                .certified_block()
                .executed_state_id(),
        )
        .unwrap();

    let proof = Proof::new(
        parent_output.frozen_subtree_roots().clone(),
        parent_output.num_leaves(),
        vec![],
    );

    let proposed_block = BlockInfo::new(
        block_epoch,
        parent.block().round(),
        parent.block().id(),
        parent_output.root_hash(),
        parent_output.version(),
        parent.block().timestamp_usecs() + 1,
        None,
    );

    let vote_data = VoteData::new(
        proposed_block,
        parent.block().quorum_cert().certified_block().clone(),
    );

    let ledger_info = match committed {
        Some(committed) => {
            let tree = committed
                .accumulator_extension_proof()
                .verify(
                    committed
                        .block()
                        .quorum_cert()
                        .certified_block()
                        .executed_state_id(),
                )
                .unwrap();
            let commit_block_info = BlockInfo::new(
                committed.block().epoch(),
                committed.block().round(),
                committed.block().id(),
                tree.root_hash(),
                tree.version(),
                committed.block().timestamp_usecs(),
                next_epoch_state,
            );
            LedgerInfo::new(commit_block_info, vote_data.hash())
        }
        None => LedgerInfo::new(BlockInfo::empty(), vote_data.hash()),
    };

    let vote = Vote::new(
        vote_data.clone(),
        validator_signer.author(),
        ledger_info,
        validator_signer,
    )
    .unwrap();

    let mut ledger_info_with_signatures = LedgerInfoWithPartialSignatures::new(
        vote.ledger_info().clone(),
        PartialSignatures::empty(),
    );

    ledger_info_with_signatures.add_signature(vote.author(), vote.signature().clone());

    let qc = QuorumCert::new(
        vote_data,
        ledger_info_with_signatures
            .aggregate_signatures(&generate_validator_verifier(&[validator_signer.clone()]))
            .unwrap(),
    );

    make_proposal_with_qc_and_proof(payload, round, proof, qc, validator_signer)
}

pub fn make_proposal_with_parent(
    payload: Payload,
    round: Round,
    parent: &VoteProposal,
    committed: Option<&VoteProposal>,
    validator_signer: &ValidatorSigner,
) -> VoteProposal {
    make_proposal_with_parent_and_overrides(
        payload,
        round,
        parent,
        committed,
        validator_signer,
        None,
        None,
    )
}

pub fn make_timeout_cert(
    round: Round,
    hqc: &QuorumCert,
    signer: &ValidatorSigner,
) -> TwoChainTimeoutCertificate {
    let timeout = TwoChainTimeout::new(1, round, hqc.clone());
    let mut tc_partial = TwoChainTimeoutWithPartialSignatures::new(timeout.clone());
    let signature = timeout.sign(signer).unwrap();
    tc_partial.add(signer.author(), timeout, signature);
    tc_partial
        .aggregate_signatures(&generate_validator_verifier(&[signer.clone()]))
        .unwrap()
}

pub fn validator_signers_to_ledger_info(signers: &[&ValidatorSigner]) -> LedgerInfo {
    let infos = signers.iter().enumerate().map(|(index, v)| {
        ValidatorInfo::new_with_test_network_keys(v.author(), v.public_key(), 1, index as u64)
    });
    let validator_set = ValidatorSet::new(infos.collect());
    LedgerInfo::mock_genesis(Some(validator_set))
}

pub fn validator_signers_to_waypoint(signers: &[&ValidatorSigner]) -> Waypoint {
    let li = validator_signers_to_ledger_info(signers);
    Waypoint::new_epoch_boundary(&li).unwrap()
}

pub fn test_storage(signer: &ValidatorSigner) -> PersistentSafetyStorage {
    let waypoint = validator_signers_to_waypoint(&[signer]);
    let storage = Storage::from(InMemoryStorage::new());
    PersistentSafetyStorage::initialize(
        storage,
        signer.author(),
        signer.private_key().clone(),
        waypoint,
        true,
    )
}

/// Returns a safety rules instance for testing purposes.
pub fn test_safety_rules() -> SafetyRules {
    let signer = ValidatorSigner::from_int(0);
    let storage = test_storage(&signer);
    let (epoch_change_proof, _) = make_genesis(&signer);

    let mut safety_rules = SafetyRules::new(storage);
    safety_rules.initialize(&epoch_change_proof).unwrap();
    safety_rules
}

/// Returns a safety rules instance that has not been initialized for testing purposes.
pub fn test_safety_rules_uninitialized() -> SafetyRules {
    let signer = ValidatorSigner::from_int(0);
    let storage = test_storage(&signer);
    SafetyRules::new(storage)
}

/// Returns a simple serializer for testing purposes.
pub fn test_serializer() -> SerializerService {
    let safety_rules = test_safety_rules();
    SerializerService::new(safety_rules)
}
