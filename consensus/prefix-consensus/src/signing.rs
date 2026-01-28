// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Signature helpers for Prefix Consensus

use crate::types::{PartyId, Vote1, Vote1SignData, Vote2, Vote2SignData, Vote3, Vote3SignData};
use anyhow::{bail, Result};
use aptos_crypto::bls12381::Signature as BlsSignature;
use aptos_types::{validator_signer::ValidatorSigner, validator_verifier::ValidatorVerifier};

/// Sign a Vote1 message
pub fn sign_vote1(vote: &Vote1, signer: &ValidatorSigner) -> Result<BlsSignature> {
    // Sign only the non-signature fields
    let sign_data = Vote1SignData {
        author: vote.author,
        input_vector: vote.input_vector.clone(),
        epoch: vote.epoch,
        slot: vote.slot,
    };
    let signature = signer.sign(&sign_data)?;
    Ok(signature)
}

/// Sign a Vote2 message
pub fn sign_vote2(vote: &Vote2, signer: &ValidatorSigner) -> Result<BlsSignature> {
    // Sign only the non-signature fields
    let sign_data = Vote2SignData {
        author: vote.author,
        certified_prefix: vote.certified_prefix.clone(),
        qc1: vote.qc1.clone(),
        epoch: vote.epoch,
        slot: vote.slot,
    };
    let signature = signer.sign(&sign_data)?;
    Ok(signature)
}

/// Sign a Vote3 message
pub fn sign_vote3(vote: &Vote3, signer: &ValidatorSigner) -> Result<BlsSignature> {
    // Sign only the non-signature fields
    let sign_data = Vote3SignData {
        author: vote.author,
        mcp_prefix: vote.mcp_prefix.clone(),
        qc2: vote.qc2.clone(),
        epoch: vote.epoch,
        slot: vote.slot,
    };
    let signature = signer.sign(&sign_data)?;
    Ok(signature)
}

/// Verify Vote1 signature
pub fn verify_vote1_signature(
    vote: &Vote1,
    author: &PartyId,
    verifier: &ValidatorVerifier,
) -> Result<()> {
    // Verify only the non-signature fields
    let sign_data = Vote1SignData {
        author: vote.author,
        input_vector: vote.input_vector.clone(),
        epoch: vote.epoch,
        slot: vote.slot,
    };
    match verifier.verify(*author, &sign_data, &vote.signature) {
        Ok(()) => Ok(()),
        Err(e) => bail!("Failed to verify Vote1 signature from {:?}: {:?}", author, e),
    }
}

/// Verify Vote2 signature
pub fn verify_vote2_signature(
    vote: &Vote2,
    author: &PartyId,
    verifier: &ValidatorVerifier,
) -> Result<()> {
    // Verify only the non-signature fields
    let sign_data = Vote2SignData {
        author: vote.author,
        certified_prefix: vote.certified_prefix.clone(),
        qc1: vote.qc1.clone(),
        epoch: vote.epoch,
        slot: vote.slot,
    };
    match verifier.verify(*author, &sign_data, &vote.signature) {
        Ok(()) => Ok(()),
        Err(e) => bail!("Failed to verify Vote2 signature from {:?}: {:?}", author, e),
    }
}

/// Verify Vote3 signature
pub fn verify_vote3_signature(
    vote: &Vote3,
    author: &PartyId,
    verifier: &ValidatorVerifier,
) -> Result<()> {
    // Verify only the non-signature fields
    let sign_data = Vote3SignData {
        author: vote.author,
        mcp_prefix: vote.mcp_prefix.clone(),
        qc2: vote.qc2.clone(),
        epoch: vote.epoch,
        slot: vote.slot,
    };
    match verifier.verify(*author, &sign_data, &vote.signature) {
        Ok(()) => Ok(()),
        Err(e) => bail!("Failed to verify Vote3 signature from {:?}: {:?}", author, e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::{bls12381, HashValue, Uniform};
    use aptos_types::{
        account_address::AccountAddress,
        validator_verifier::ValidatorConsensusInfo,
    };
    use std::sync::Arc;

    fn create_test_signer_and_verifier() -> (ValidatorSigner, ValidatorVerifier) {
        let private_key = bls12381::PrivateKey::generate_for_testing();
        let public_key = bls12381::PublicKey::from(&private_key);
        let author = AccountAddress::random();

        let signer = ValidatorSigner::new(author, Arc::new(private_key));

        // Verify that the signer's public key matches
        eprintln!("Generated public key: {:?}", public_key);
        eprintln!("Signer's public key: {:?}", signer.public_key());
        assert_eq!(public_key, signer.public_key(), "Public keys should match");

        let validator_consensus_info = ValidatorConsensusInfo::new(author, public_key, 1);
        let verifier = ValidatorVerifier::new(vec![validator_consensus_info]);

        (signer, verifier)
    }

    #[test]
    fn test_sign_and_verify_vote1() {
        let (signer, verifier) = create_test_signer_and_verifier();
        let author = signer.author();

        // Create a Vote1 with dummy signature first
        let input_vector = vec![HashValue::random(), HashValue::random()];
        let dummy_sig = bls12381::Signature::dummy_signature();
        let mut vote = Vote1::new(author, input_vector, 0, 0, dummy_sig.clone());

        // Compute hash before signing
        let hash_before = aptos_crypto::hash::CryptoHash::hash(&vote);
        eprintln!("Hash before signing: {:?}", hash_before);

        // Sign it
        let signature = sign_vote1(&vote, &signer).unwrap();
        vote.signature = signature;

        // Compute hash after signing (should be the same since we exclude signature)
        let hash_after = aptos_crypto::hash::CryptoHash::hash(&vote);
        eprintln!("Hash after signing: {:?}", hash_after);
        assert_eq!(hash_before, hash_after, "Hash should be the same before and after signing");

        // Verify it
        let result = verify_vote1_signature(&vote, &author, &verifier);
        if result.is_err() {
            eprintln!("Verification failed: {:?}", result);
            eprintln!("Author: {:?}", author);
        }
        assert!(result.is_ok(), "Vote1 signature verification should pass");
    }

    #[test]
    fn test_sign_and_verify_vote2() {
        let (signer, verifier) = create_test_signer_and_verifier();
        let author = signer.author();

        // Create a Vote2 with dummy QC1 and signature
        let certified_prefix = vec![HashValue::random()];
        let dummy_sig = bls12381::Signature::dummy_signature();
        let qc1_vote = Vote1::new(author, vec![], 0, 0, dummy_sig.clone());
        let qc1 = crate::types::QC1::new(vec![qc1_vote]);

        let mut vote = Vote2::new(author, certified_prefix, qc1, 0, 0, dummy_sig);

        // Sign it
        let signature = sign_vote2(&vote, &signer).unwrap();
        vote.signature = signature;

        // Verify it
        let result = verify_vote2_signature(&vote, &author, &verifier);
        assert!(result.is_ok(), "Vote2 signature verification should pass");
    }

    #[test]
    fn test_sign_and_verify_vote3() {
        let (signer, verifier) = create_test_signer_and_verifier();
        let author = signer.author();

        // Create a Vote3 with dummy QC2 and signature
        let mcp_prefix = vec![HashValue::random()];
        let dummy_sig = bls12381::Signature::dummy_signature();

        // Create nested QCs
        let qc1_vote = Vote1::new(author, vec![], 0, 0, dummy_sig.clone());
        let qc1 = crate::types::QC1::new(vec![qc1_vote]);
        let qc2_vote = Vote2::new(author, vec![], qc1, 0, 0, dummy_sig.clone());
        let qc2 = crate::types::QC2::new(vec![qc2_vote]);

        let mut vote = Vote3::new(author, mcp_prefix, qc2, 0, 0, dummy_sig);

        // Sign it
        let signature = sign_vote3(&vote, &signer).unwrap();
        vote.signature = signature;

        // Verify it
        let result = verify_vote3_signature(&vote, &author, &verifier);
        assert!(result.is_ok(), "Vote3 signature verification should pass");
    }

    #[test]
    fn test_verify_invalid_signature_fails() {
        let (signer, verifier) = create_test_signer_and_verifier();
        let author = signer.author();

        // Create a Vote1 with wrong signature
        let input_vector = vec![HashValue::random()];
        let wrong_sig = bls12381::Signature::dummy_signature();
        let vote = Vote1::new(author, input_vector, 0, 0, wrong_sig);

        // Verification should fail
        let result = verify_vote1_signature(&vote, &author, &verifier);
        assert!(result.is_err(), "Invalid signature should fail verification");
    }
}
