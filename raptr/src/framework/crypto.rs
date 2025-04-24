// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::framework::NodeId;
use aptos_crypto::{
    bls12381::{self, PublicKey},
    hash::CryptoHash,
    Genesis, PrivateKey, Signature, SigningKey, Uniform,
};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{validator_signer::ValidatorSigner, validator_verifier::ValidatorVerifier};
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// FIXME: for testing and prototyping only, obviously not safe in prod.
fn deterministic_tag_private_keys(node_id: usize, n_tags: usize) -> Vec<bls12381::PrivateKey> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(node_id as u64);
    (0..n_tags)
        .map(|_| bls12381::PrivateKey::generate(&mut rng))
        .collect()
}

#[cfg(feature = "wendy")]
fn deterministic_wendy_private_keys(
    node_id: usize,
    n_tags: usize,
) -> Vec<(bls12381::PrivateKey, bls12381::PrivateKey)> {
    let n_bits = if n_tags == 0 {
        0
    } else {
        (n_tags - 1).next_power_of_two().ilog2()
    };

    let mut rng = rand::rngs::StdRng::seed_from_u64(node_id as u64);
    (0..n_bits)
        .map(|_| {
            (
                bls12381::PrivateKey::generate(&mut rng),
                bls12381::PrivateKey::generate(&mut rng),
            )
        })
        .collect()
}

#[derive(Clone)]
pub struct SignatureVerifier {
    inner: Arc<VerifierInner>,
}

struct VerifierInner {
    public_keys: Vec<PublicKey>,
    tag_public_keys: Vec<Vec<PublicKey>>,
    // For compatibility with aptos codebase.
    aptos_verifier: Arc<ValidatorVerifier>,

    #[cfg(feature = "wendy")]
    wendy_public_keys: Vec<Vec<(PublicKey, PublicKey)>>,
}

impl SignatureVerifier {
    pub fn new(
        public_keys: Vec<PublicKey>,
        aptos_verifier: Arc<ValidatorVerifier>,
        n_tags: usize,
    ) -> Self {
        let tag_public_keys = (0..public_keys.len())
            .into_iter()
            .map(|node_id| {
                deterministic_tag_private_keys(node_id, n_tags)
                    .into_iter()
                    .map(|private_key| private_key.public_key())
                    .collect()
            })
            .collect();

        #[cfg(feature = "wendy")]
        let wendy_public_keys = (0..public_keys.len())
            .into_iter()
            .map(|node_id| {
                deterministic_wendy_private_keys(node_id, n_tags)
                    .into_iter()
                    .map(|(key0, key1)| (key0.public_key(), key1.public_key()))
                    .collect()
            })
            .collect();

        SignatureVerifier {
            inner: Arc::new(VerifierInner {
                public_keys,
                tag_public_keys,
                aptos_verifier,

                #[cfg(feature = "wendy")]
                wendy_public_keys,
            }),
        }
    }

    /// Verify the correctness of a signature of a message by a known author.
    pub fn verify<T: Serialize + CryptoHash>(
        &self,
        author: NodeId,
        message: &T,
        signature: &bls12381::Signature,
    ) -> anyhow::Result<()> {
        signature.verify(message, &self.inner.public_keys[author])
    }

    pub fn verify_aggregate_signatures<T: CryptoHash + Serialize>(
        &self,
        nodes: impl IntoIterator<Item = NodeId>,
        messages: Vec<&T>,
        signature: &bls12381::Signature,
    ) -> anyhow::Result<()> {
        let public_keys: Vec<_> = nodes
            .into_iter()
            .map(|node| &self.inner.public_keys[node])
            .collect();

        signature.verify_aggregate(&messages, &public_keys)
    }

    pub fn verify_multi_signature<T: CryptoHash + Serialize>(
        &self,
        nodes: impl IntoIterator<Item = NodeId>,
        message: &T,
        multi_sig: &bls12381::Signature,
    ) -> anyhow::Result<()> {
        let pub_keys: Vec<_> = nodes
            .into_iter()
            .map(|node| &self.inner.public_keys[node])
            .collect();

        let aggregated_key = PublicKey::aggregate(pub_keys)?;

        multi_sig.verify(message, &aggregated_key)
    }

    pub fn verify_tagged<T: CryptoHash + Serialize>(
        &self,
        author: NodeId,
        message: &T,
        tag: usize,
        signature: &bls12381::Signature,
    ) -> anyhow::Result<()> {
        signature.verify(message, &self.inner.tag_public_keys[author][tag])
    }

    pub fn verify_tagged_multi_signature<T: CryptoHash + Serialize>(
        &self,
        nodes: impl IntoIterator<Item = NodeId>,
        message: &T,
        tags: impl IntoIterator<Item = usize>,
        signature: &bls12381::Signature,
    ) -> anyhow::Result<()> {
        let pub_keys: Vec<_> = nodes
            .into_iter()
            .zip(tags.into_iter())
            .map(|(node, tag)| &self.inner.tag_public_keys[node][tag])
            .collect();

        let aggregated_key = PublicKey::aggregate(pub_keys)?;
        signature.verify(message, &aggregated_key)
    }

    #[cfg(feature = "wendy")]
    fn get_wendy_public_keys(
        &self,
        author: NodeId,
        tag: usize,
    ) -> impl Iterator<Item = &PublicKey> {
        self.inner.wendy_public_keys[author]
            .iter()
            .enumerate()
            .map(
                move |(bit, (key0, key1))| {
                    if tag & (1 << bit) == 0 {
                        key0
                    } else {
                        key1
                    }
                },
            )
    }

    #[cfg(feature = "wendy")]
    pub fn verify_wendy<T: CryptoHash + Serialize>(
        &self,
        author: NodeId,
        message: &T,
        tag: usize,
        signature: &bls12381::Signature,
    ) -> anyhow::Result<()> {
        let public_key = PublicKey::aggregate(self.get_wendy_public_keys(author, tag).collect())?;
        signature.verify(message, &public_key)
    }

    #[cfg(feature = "wendy")]
    pub fn verify_wendy_multi_signature<T: CryptoHash + Serialize>(
        &self,
        nodes: impl IntoIterator<Item = NodeId>,
        message: &T,
        tags: impl IntoIterator<Item = usize>,
        signature: &bls12381::Signature,
    ) -> anyhow::Result<()> {
        let pub_keys = nodes
            .into_iter()
            .zip(tags.into_iter())
            .flat_map(|(node, tag)| self.get_wendy_public_keys(node, tag))
            .collect();

        let aggregated_key = PublicKey::aggregate(pub_keys)?;
        signature.verify(message, &aggregated_key)
    }

    pub fn aggregate_signatures(
        &self,
        partial_signatures: impl IntoIterator<Item = bls12381::Signature>,
    ) -> anyhow::Result<bls12381::Signature> {
        let signatures = partial_signatures.into_iter().collect();

        bls12381::Signature::aggregate(signatures)
    }

    pub fn aptos_verifier(&self) -> &ValidatorVerifier {
        &self.inner.aptos_verifier
    }
}

#[derive(Clone)]
pub struct Signer {
    inner: Arc<SignerInner>,
}

struct SignerInner {
    // A hack to be compatible with aptos codebase.
    // ValidatorSigner does not expose the private key.
    aptos_signer: Arc<ValidatorSigner>,
    tag_private_keys: Vec<bls12381::PrivateKey>,

    #[cfg(feature = "wendy")]
    wendy_private_keys: Vec<(bls12381::PrivateKey, bls12381::PrivateKey)>,
}

impl Signer {
    pub fn new(aptos_signer: Arc<ValidatorSigner>, node_id: NodeId, n_tags: usize) -> Self {
        Signer {
            inner: Arc::new(SignerInner {
                aptos_signer,
                tag_private_keys: deterministic_tag_private_keys(node_id, n_tags),

                #[cfg(feature = "wendy")]
                wendy_private_keys: deterministic_wendy_private_keys(node_id, n_tags),
            }),
        }
    }

    pub fn sign<T: Serialize + CryptoHash>(
        &self,
        message: &T,
    ) -> anyhow::Result<bls12381::Signature> {
        Ok(self.inner.aptos_signer.sign(message)?)
    }

    pub fn sign_tagged<T: Serialize + CryptoHash>(
        &self,
        message: &T,
        tag: usize,
    ) -> anyhow::Result<bls12381::Signature> {
        Ok(self.inner.tag_private_keys[tag].sign(message)?)
    }

    #[cfg(feature = "wendy")]
    pub fn sign_wendy<T: Serialize + CryptoHash>(
        &self,
        message: &T,
        tag: usize,
    ) -> anyhow::Result<bls12381::Signature> {
        let mut sigs = Vec::new();

        for (bit, (key0, key1)) in self.inner.wendy_private_keys.iter().enumerate() {
            if tag & (1 << bit) == 0 {
                sigs.push(key0.sign(message)?);
            } else {
                sigs.push(key1.sign(message)?);
            }
        }

        Ok(bls12381::Signature::aggregate(sigs)?)
    }
}

/// Returns a nonsense signature.
/// Used as a placeholder.
pub fn dummy_signature() -> bls12381::Signature {
    static SIGNATURE: std::sync::OnceLock<bls12381::Signature> = std::sync::OnceLock::new();

    #[derive(CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
    struct DummyMessage {}

    SIGNATURE
        .get_or_init(|| {
            let private_key = bls12381::PrivateKey::genesis();
            private_key.sign(&DummyMessage {}).unwrap()
        })
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::hash::CryptoHash;
    use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
    use rand::{rngs::StdRng, SeedableRng};
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;

    /// A simple test message.
    /// The derives enable both BCS hashing and Serde serialization.
    #[derive(CryptoHasher, BCSCryptoHash, Serialize, Deserialize, Debug, PartialEq)]
    struct TestMessage {
        value: u64,
    }

    /// Helper: Create a deterministic private key for testing purposes.
    /// This is used for aggregate and multi-signature tests.
    fn deterministic_main_private_key(node_id: usize) -> bls12381::PrivateKey {
        let mut seed = [0u8; 32];
        seed[..8].copy_from_slice(&node_id.to_le_bytes());
        let mut rng = StdRng::from_seed(seed);
        bls12381::PrivateKey::generate(&mut rng)
    }

    // ============================
    // Positive Tests
    // ============================

    /// Verify that a tagged signature (created via `Signer::sign_tagged`)
    /// is correctly verified.
    #[test]
    fn test_tagged_signature_verification() -> anyhow::Result<()> {
        let n_tags = 3;
        let node_id = 0;
        let vs = ValidatorSigner::random(None);

        // Use the validator signer's public key.
        let public_keys = vec![vs.public_key()];
        let signer = Signer::new(Arc::new(vs), node_id, n_tags);
        let msg = TestMessage { value: 42 };

        let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
        let signature_verifier = SignatureVerifier::new(public_keys, dummy_verifier, n_tags);

        // Sign and verify for each tag.
        for tag in 0..n_tags {
            let sig = signer.sign_tagged(&msg, tag)?;
            signature_verifier.verify_tagged(node_id, &msg, tag, &sig)?;
        }
        Ok(())
    }

    /// Verify that multiple tagged signatures (from different nodes and tags)
    /// can be aggregated and verified as a tagged multi-signature.
    #[test]
    fn test_tagged_multi_signature_verification() -> anyhow::Result<()> {
        let n_tags = 3;
        let num_nodes: usize = 3;
        let mut signers = Vec::new();
        let mut public_keys = Vec::new();

        // Create a signer for each node.
        for node_id in 0..num_nodes {
            let mut seed = [0u8; 32];
            seed[..8].copy_from_slice(&node_id.to_le_bytes());
            let vs = ValidatorSigner::random(seed);
            public_keys.push(vs.public_key());
            signers.push(Signer::new(Arc::new(vs), node_id, n_tags));
        }

        let msg = TestMessage { value: 100 };

        // Let each node use a tag equal to (node_id mod n_tags).
        let tags: Vec<usize> = (0..num_nodes).map(|node_id| node_id % n_tags).collect();

        // Each signer signs with its corresponding tag.
        let mut sigs = Vec::new();
        for (node_id, signer) in signers.iter().enumerate() {
            let tag = tags[node_id];
            let sig = signer.sign_tagged(&msg, tag)?;
            sigs.push(sig);
        }

        // Aggregate the signatures.
        let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
        let signature_verifier = SignatureVerifier::new(public_keys, dummy_verifier, n_tags);
        let aggregated_sig = signature_verifier.aggregate_signatures(sigs)?;

        // Verify the aggregated tagged multi-signature.
        signature_verifier.verify_tagged_multi_signature(
            0..num_nodes,
            &msg,
            tags,
            &aggregated_sig,
        )?;
        Ok(())
    }

    /// Verify that a “normal” (non-tagged) signature is correctly verified.
    #[test]
    fn test_non_tagged_signature_verification() -> anyhow::Result<()> {
        let vs = ValidatorSigner::random(None);
        let node_id = 0;
        let msg = TestMessage { value: 7 };
        let sig = vs.sign(&msg)?;
        let public_keys = vec![vs.public_key()];
        let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
        let signature_verifier = SignatureVerifier::new(public_keys, dummy_verifier, 1);
        signature_verifier.verify(node_id, &msg, &sig)?;
        Ok(())
    }

    /// Verify aggregate signature verification for a set of (different) messages,
    /// where only a subset (7 out of 10) of the nodes participate.
    #[test]
    fn test_aggregate_signature_verification() -> anyhow::Result<()> {
        let total_nodes = 10;
        // Define participating nodes (7 out of 10).
        let participating_nodes = vec![0, 2, 4, 6, 7, 8, 9];

        let mut msgs = Vec::new();
        let mut individual_sigs = Vec::new();
        let mut public_keys = Vec::new();

        // Create distinct messages for each node.
        for node_id in 0..total_nodes {
            msgs.push(TestMessage {
                value: node_id as u64,
            });
        }

        // Sign each message using a deterministic private key.
        for node_id in 0..total_nodes {
            let private_key = deterministic_main_private_key(node_id);
            let sig = private_key.sign(&msgs[node_id])?;
            individual_sigs.push(sig);
            public_keys.push(private_key.public_key());
        }

        let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
        let signature_verifier = SignatureVerifier::new(public_keys, dummy_verifier, 1);

        // Only consider messages and signatures from participating nodes.
        let participating_msgs: Vec<&TestMessage> =
            participating_nodes.iter().map(|&i| &msgs[i]).collect();
        let participating_sigs: Vec<bls12381::Signature> = participating_nodes
            .iter()
            .map(|&i| individual_sigs[i].clone())
            .collect();

        // Aggregate the partial signatures.
        let aggregated_sig = signature_verifier.aggregate_signatures(participating_sigs)?;

        // Verify the aggregated signature.
        signature_verifier.verify_aggregate_signatures(
            participating_nodes,
            participating_msgs,
            &aggregated_sig,
        )?;
        Ok(())
    }

    /// Verify multi-signature verification where all nodes sign the same message,
    /// but only a subset (7 out of 10) of the nodes participate.
    #[test]
    fn test_multi_signature_verification() -> anyhow::Result<()> {
        let total_nodes = 10;
        // Define participating nodes (7 out of 10).
        let participating_nodes = vec![1, 3, 4, 6, 7, 8, 9];
        let msg = TestMessage { value: 999 };

        let mut individual_sigs = Vec::new();
        let mut public_keys = Vec::new();

        // All nodes sign the same message.
        for node_id in 0..total_nodes {
            let private_key = deterministic_main_private_key(node_id);
            let sig = private_key.sign(&msg)?;
            individual_sigs.push(sig);
            public_keys.push(private_key.public_key());
        }

        let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
        let signature_verifier = SignatureVerifier::new(public_keys, dummy_verifier, 1);

        // Filter the signatures for participating nodes.
        let participating_sigs: Vec<bls12381::Signature> = participating_nodes
            .iter()
            .map(|&i| individual_sigs[i].clone())
            .collect();

        // Aggregate the signatures.
        let aggregated_sig = signature_verifier.aggregate_signatures(participating_sigs)?;

        // Verify the multi-signature.
        signature_verifier.verify_multi_signature(participating_nodes, &msg, &aggregated_sig)?;
        Ok(())
    }

    /// Verify tagged multi-signature verification where only a subset (5 out of 10)
    /// of the nodes participate.
    #[test]
    fn test_tagged_multi_signature_verification_subset() -> anyhow::Result<()> {
        let total_nodes: usize = 10;
        let n_tags = 3;

        // Create signers and collect public keys.
        let mut signers = Vec::new();
        let mut public_keys = Vec::new();
        for node_id in 0..total_nodes {
            let mut seed = [0u8; 32];
            seed[..8].copy_from_slice(&node_id.to_le_bytes());
            let vs = ValidatorSigner::random(seed);
            public_keys.push(vs.public_key());
            signers.push(Signer::new(Arc::new(vs), node_id, n_tags));
        }

        let msg = TestMessage { value: 500 };

        // Select a subset of participating nodes.
        let participating_nodes = vec![1, 3, 5, 7, 9];

        // Each participating node uses a tag (node_id mod n_tags).
        let tags: Vec<usize> = participating_nodes
            .iter()
            .map(|&node_id| node_id % n_tags)
            .collect();

        let mut sigs = Vec::new();
        for &node_id in &participating_nodes {
            let signer = &signers[node_id];
            let tag = node_id % n_tags;
            let sig = signer.sign_tagged(&msg, tag)?;
            sigs.push(sig);
        }

        let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
        let signature_verifier = SignatureVerifier::new(public_keys, dummy_verifier, n_tags);
        let aggregated_sig = signature_verifier.aggregate_signatures(sigs)?;

        // Verify the tagged multi-signature.
        signature_verifier.verify_tagged_multi_signature(
            participating_nodes,
            &msg,
            tags,
            &aggregated_sig,
        )?;
        Ok(())
    }

    // ============================
    // Negative Tests
    // ============================

    /// Aggregate signature negative test:
    /// Verification should fail if one of the messages is altered.
    #[test]
    fn test_aggregate_signature_negative() -> anyhow::Result<()> {
        let total_nodes = 10;
        let participating_nodes = vec![0, 2, 4, 6, 7, 8, 9];

        let mut msgs = Vec::new();
        let mut individual_sigs = Vec::new();
        let mut public_keys = Vec::new();

        // Create messages for all nodes.
        for node_id in 0..total_nodes {
            msgs.push(TestMessage {
                value: node_id as u64,
            });
        }

        // Sign each message.
        for node_id in 0..total_nodes {
            let private_key = deterministic_main_private_key(node_id);
            let sig = private_key.sign(&msgs[node_id])?;
            individual_sigs.push(sig);
            public_keys.push(private_key.public_key());
        }

        let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
        let signature_verifier = SignatureVerifier::new(public_keys, dummy_verifier, 1);

        let participating_msgs: Vec<&TestMessage> =
            participating_nodes.iter().map(|&i| &msgs[i]).collect();
        let participating_sigs: Vec<bls12381::Signature> = participating_nodes
            .iter()
            .map(|&i| individual_sigs[i].clone())
            .collect();

        let aggregated_sig = signature_verifier.aggregate_signatures(participating_sigs)?;

        // Deliberately alter one message.
        let mut wrong_msgs = participating_msgs.clone();
        if let Some(first_msg) = wrong_msgs.get_mut(0) {
            *first_msg = &TestMessage { value: 9999 };
        }

        let result = signature_verifier.verify_aggregate_signatures(
            participating_nodes.clone(),
            wrong_msgs,
            &aggregated_sig,
        );
        assert!(
            result.is_err(),
            "Aggregate signature verification should fail when messages are altered"
        );
        Ok(())
    }

    /// Multi-signature negative test:
    /// Verification should fail when using the wrong message.
    #[test]
    fn test_multi_signature_negative() -> anyhow::Result<()> {
        let total_nodes = 10;
        let participating_nodes = vec![1, 3, 4, 6, 7, 8, 9];
        let msg = TestMessage { value: 999 };

        let mut individual_sigs = Vec::new();
        let mut public_keys = Vec::new();

        // All nodes sign the same message.
        for node_id in 0..total_nodes {
            let private_key = deterministic_main_private_key(node_id);
            let sig = private_key.sign(&msg)?;
            individual_sigs.push(sig);
            public_keys.push(private_key.public_key());
        }

        let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
        let signature_verifier = SignatureVerifier::new(public_keys, dummy_verifier, 1);

        let participating_sigs: Vec<bls12381::Signature> = participating_nodes
            .iter()
            .map(|&i| individual_sigs[i].clone())
            .collect();

        let aggregated_sig = signature_verifier.aggregate_signatures(participating_sigs)?;

        // Use a wrong message for verification.
        let wrong_msg = TestMessage { value: 1234 };
        let result = signature_verifier.verify_multi_signature(
            participating_nodes,
            &wrong_msg,
            &aggregated_sig,
        );
        assert!(
            result.is_err(),
            "Multi-signature verification should fail when using the wrong message"
        );
        Ok(())
    }

    /// Tagged multi-signature negative test:
    /// Verification should fail if an incorrect tag vector is provided.
    #[test]
    fn test_tagged_multi_signature_negative() -> anyhow::Result<()> {
        let total_nodes: usize = 10;
        let n_tags = 3;

        let mut signers = Vec::new();
        let mut public_keys = Vec::new();
        for node_id in 0..total_nodes {
            let mut seed = [0u8; 32];
            seed[..8].copy_from_slice(&node_id.to_le_bytes());
            let vs = ValidatorSigner::random(seed);
            public_keys.push(vs.public_key());
            signers.push(Signer::new(Arc::new(vs), node_id, n_tags));
        }

        let msg = TestMessage { value: 500 };
        let participating_nodes = vec![1, 3, 5, 7, 9];

        // Build the correct tags vector...
        let mut correct_tags: Vec<usize> = participating_nodes
            .iter()
            .map(|&node_id| node_id % n_tags)
            .collect();
        // ...and then modify one tag to be incorrect.
        if let Some(first) = correct_tags.get_mut(0) {
            *first = (*first + 1) % n_tags;
        }

        let mut sigs = Vec::new();
        // Each signer signs with the proper (correct) tag.
        for &node_id in &participating_nodes {
            let signer = &signers[node_id];
            let correct_tag = node_id % n_tags;
            let sig = signer.sign_tagged(&msg, correct_tag)?;
            sigs.push(sig);
        }

        let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
        let signature_verifier = SignatureVerifier::new(public_keys, dummy_verifier, n_tags);
        let aggregated_sig = signature_verifier.aggregate_signatures(sigs)?;

        // Verification using the modified (incorrect) tag vector should fail.
        let result = signature_verifier.verify_tagged_multi_signature(
            participating_nodes,
            &msg,
            correct_tags,
            &aggregated_sig,
        );
        assert!(
            result.is_err(),
            "Tagged multi-signature verification should fail when tag indices are incorrect"
        );
        Ok(())
    }

    /// Test that a single “wendy” signature (created via `Signer::sign_wendy`)
    /// is correctly verified using `SignatureVerifier::verify_wendy`.
    #[cfg(feature = "wendy")]
    #[test]
    fn test_wendy_signature_verification() -> anyhow::Result<()> {
        // Using n_tags = 3 will cause `deterministic_wendy_private_keys` to generate
        // 1 key pair (since (3-1).next_power_of_two().ilog2() == 1), meaning that valid
        // wendy tag values are 0 or 1.
        let n_tags = 3;
        let node_id = 0;
        let vs = ValidatorSigner::random(None);
        let public_keys = vec![vs.public_key()];
        let signer = Signer::new(Arc::new(vs), node_id, n_tags);
        let msg = TestMessage { value: 42 };

        let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
        let signature_verifier = SignatureVerifier::new(public_keys, dummy_verifier, n_tags);

        // Try both valid wendy tags: 0 and 1.
        for tag in 0..2 {
            let sig = signer.sign_wendy(&msg, tag)?;
            signature_verifier.verify_wendy(node_id, &msg, tag, &sig)?;
        }
        Ok(())
    }

    /// Test that wendy multi-signatures (from multiple nodes signing the same message)
    /// can be aggregated and verified using `verify_wendy_multi_signature`.
    #[cfg(feature = "wendy")]
    #[test]
    fn test_wendy_multi_signature_verification() -> anyhow::Result<()> {
        let total_nodes: usize = 5;
        let n_tags = 3; // yields 1 wendy key pair per node (valid tags: 0 or 1)
        let mut signers = Vec::new();
        let mut public_keys = Vec::new();

        // Create signers and collect public keys.
        for node_id in 0..total_nodes {
            let mut seed = [0u8; 32];
            seed[..8].copy_from_slice(&node_id.to_le_bytes());
            let vs = ValidatorSigner::random(Some(seed));
            public_keys.push(vs.public_key());
            signers.push(Signer::new(Arc::new(vs), node_id, n_tags));
        }

        let msg = TestMessage { value: 100 };

        // For each signer, choose a valid wendy tag.
        // Here we simply use node_id % 2 (ensuring a value of 0 or 1).
        let tags: Vec<usize> = (0..total_nodes).map(|node_id| node_id % 2).collect();

        // Each signer signs using `sign_wendy`.
        let mut sigs = Vec::new();
        for (node_id, signer) in signers.iter().enumerate() {
            let tag = tags[node_id];
            let sig = signer.sign_wendy(&msg, tag)?;
            sigs.push(sig);
        }

        let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
        let signature_verifier = SignatureVerifier::new(public_keys, dummy_verifier, n_tags);

        // Aggregate the individual wendy signatures.
        let aggregated_sig = signature_verifier.aggregate_signatures(sigs)?;
        // Verify the aggregated multi-signature.
        signature_verifier.verify_wendy_multi_signature(
            0..total_nodes,
            &msg,
            tags,
            &aggregated_sig,
        )?;
        Ok(())
    }

    /// Negative test: Verify that a wendy signature fails when a wrong message is provided.
    #[cfg(feature = "wendy")]
    #[test]
    fn test_wendy_signature_wrong_message() -> anyhow::Result<()> {
        let n_tags = 3;
        let node_id = 0;
        let vs = ValidatorSigner::random(None);
        let public_keys = vec![vs.public_key()];
        let signer = Signer::new(Arc::new(vs), node_id, n_tags);
        let msg = TestMessage { value: 42 };
        let wrong_msg = TestMessage { value: 43 };

        let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
        let signature_verifier = SignatureVerifier::new(public_keys, dummy_verifier, n_tags);

        let sig = signer.sign_wendy(&msg, 0)?;
        let result = signature_verifier.verify_wendy(node_id, &wrong_msg, 0, &sig);
        assert!(
            result.is_err(),
            "Wendy signature verification should fail with a wrong message"
        );
        Ok(())
    }

    /// Negative test: Verify that a wendy multi-signature fails when an incorrect tag vector is provided.
    #[cfg(feature = "wendy")]
    #[test]
    fn test_wendy_multi_signature_negative() -> anyhow::Result<()> {
        let total_nodes: usize = 5;
        let n_tags = 3;
        let mut signers = Vec::new();
        let mut public_keys = Vec::new();

        for node_id in 0..total_nodes {
            let mut seed = [0u8; 32];
            seed[..8].copy_from_slice(&node_id.to_le_bytes());
            let vs = ValidatorSigner::random(Some(seed));
            public_keys.push(vs.public_key());
            signers.push(Signer::new(Arc::new(vs), node_id, n_tags));
        }

        let msg = TestMessage { value: 500 };

        // Build the correct tags vector (each signer uses node_id % 2).
        let mut correct_tags: Vec<usize> = (0..total_nodes).map(|node_id| node_id % 2).collect();
        // Modify one tag to be incorrect.
        if let Some(first) = correct_tags.get_mut(0) {
            *first = (*first + 1) % 2;
        }

        let mut sigs = Vec::new();
        // Each signer signs with its proper (correct) wendy tag.
        for node_id in 0..total_nodes {
            let signer = &signers[node_id];
            let proper_tag = node_id % 2;
            let sig = signer.sign_wendy(&msg, proper_tag)?;
            sigs.push(sig);
        }

        let dummy_verifier = Arc::new(ValidatorVerifier::new(vec![]));
        let signature_verifier = SignatureVerifier::new(public_keys, dummy_verifier, n_tags);
        let aggregated_sig = signature_verifier.aggregate_signatures(sigs)?;

        let result = signature_verifier.verify_wendy_multi_signature(
            0..total_nodes,
            &msg,
            correct_tags,
            &aggregated_sig,
        );
        assert!(
            result.is_err(),
            "Wendy multi-signature verification should fail when tag indices are incorrect"
        );
        Ok(())
    }
}
