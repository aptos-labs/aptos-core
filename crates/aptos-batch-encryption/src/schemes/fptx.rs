// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::{
    errors::MissingEvalProofError,
    group::*,
    shared::{
        ciphertext::{CTDecrypt, CTEncrypt, PreparedCiphertext, StandardCiphertext},
        digest::{Digest, DigestKey, EvalProof, EvalProofs, EvalProofsPromise},
        encryption_key::EncryptionKey,
        ids::{Id, IdSet, UncomputedCoeffs},
        key_derivation::{
            self, BIBEDecryptionKey, BIBEDecryptionKeyShare, BIBEMasterSecretKeyShare,
            BIBEVerificationKey,
        },
    },
    traits::{AssociatedData, BatchThresholdEncryption, Plaintext},
};
use anyhow::Result;
use aptos_dkg::pvss::{
    traits::{Reconstructable as _, TranscriptCore},
    Player,
};
use ark_ff::UniformRand as _;
use ark_std::rand::{rngs::StdRng, CryptoRng, RngCore, SeedableRng};

pub struct FPTX {}

impl BatchThresholdEncryption for FPTX {
    type Ciphertext = StandardCiphertext;
    type DecryptionKey = BIBEDecryptionKey;
    type DecryptionKeyShare = BIBEDecryptionKeyShare;
    type Digest = Digest;
    type DigestKey = DigestKey;
    type EncryptionKey = EncryptionKey;
    type EvalProof = EvalProof;
    type EvalProofs = EvalProofs;
    type EvalProofsPromise = EvalProofsPromise;
    type Id = Id;
    type MasterSecretKeyShare = BIBEMasterSecretKeyShare;
    type PreparedCiphertext = PreparedCiphertext;
    type Round = u64;
    // This is essentially a placeholder, since there is no PVSS scheme right now that works
    // with the unweighted `SmairThresholdConfig`
    type SubTranscript = aptos_dkg::pvss::chunky::WeightedSubtranscript<Pairing>;
    type ThresholdConfig = aptos_crypto::arkworks::shamir::ShamirThresholdConfig<Fr>;
    type VerificationKey = BIBEVerificationKey;

    fn setup(
        _digest_key: &Self::DigestKey,
        _pvss_public_params: &<Self::SubTranscript as TranscriptCore>::PublicParameters,
        _subtranscript: &Self::SubTranscript,
        _threshold_config: &Self::ThresholdConfig,
        _current_player: Player,
        _msk_share_decryption_key: &<Self::SubTranscript as TranscriptCore>::DecryptPrivKey,
    ) -> Result<(
        Self::EncryptionKey,
        Vec<Self::VerificationKey>,
        Self::MasterSecretKeyShare,
    )> {
        // B/c unweighted chunky is being removed
        unimplemented!()
    }

    fn extract_encryption_key(
        _digest_key: &Self::DigestKey,
        _subtranscript: &Self::SubTranscript,
    ) -> Result<Self::EncryptionKey> {
        // B/c unweighted chunky is being removed
        unimplemented!()
    }

    fn setup_for_testing(
        seed: u64,
        max_batch_size: usize,
        number_of_rounds: usize,
        threshold_config: &Self::ThresholdConfig,
    ) -> Result<(
        Self::EncryptionKey,
        Self::DigestKey,
        Vec<Self::VerificationKey>,
        Vec<Self::MasterSecretKeyShare>,
    )> {
        let mut rng = <StdRng as SeedableRng>::seed_from_u64(seed);

        let digest_key = DigestKey::new(&mut rng, max_batch_size, number_of_rounds)?;
        let msk = Fr::rand(&mut rng);
        let (mpk, vks, msk_shares) =
            key_derivation::gen_msk_shares(msk, &mut rng, threshold_config);

        let ek = EncryptionKey::new(mpk, digest_key.tau_g2);

        Ok((ek, digest_key, vks, msk_shares))
    }

    fn encrypt<R: CryptoRng + RngCore>(
        ek: &Self::EncryptionKey,
        rng: &mut R,
        msg: &impl Plaintext,
        associated_data: &impl AssociatedData,
    ) -> anyhow::Result<Self::Ciphertext> {
        ek.encrypt(rng, msg, associated_data)
    }

    fn digest(
        digest_key: &Self::DigestKey,
        cts: &[Self::Ciphertext],
        round: Self::Round,
    ) -> anyhow::Result<(Self::Digest, Self::EvalProofsPromise)> {
        let mut ids: IdSet<UncomputedCoeffs> =
            IdSet::from_slice(&cts.iter().map(|ct| ct.id()).collect::<Vec<Id>>());

        digest_key.digest(&mut ids, round)
    }

    fn verify_ct(
        ct: &Self::Ciphertext,
        associated_data: &impl AssociatedData,
    ) -> anyhow::Result<()> {
        ct.verify(associated_data)
    }

    fn ct_id(ct: &Self::Ciphertext) -> Self::Id {
        ct.id()
    }

    fn eval_proofs_compute_all(
        proofs: &Self::EvalProofsPromise,
        digest_key: &DigestKey,
    ) -> Self::EvalProofs {
        proofs.compute_all(digest_key)
    }

    fn eval_proofs_compute_all_vzgg_multi_point_eval(
        proofs: &Self::EvalProofsPromise,
        digest_key: &DigestKey,
    ) -> Self::EvalProofs {
        proofs.compute_all_vgzz_multi_point_eval(digest_key)
    }

    fn eval_proof_for_ct(
        proofs: &Self::EvalProofs,
        ct: &Self::Ciphertext,
    ) -> Option<Self::EvalProof> {
        proofs.get(&ct.id())
    }

    fn derive_decryption_key_share(
        msk_share: &Self::MasterSecretKeyShare,
        digest: &Self::Digest,
    ) -> Result<Self::DecryptionKeyShare> {
        msk_share.derive_decryption_key_share(digest)
    }

    fn reconstruct_decryption_key(
        shares: &[Self::DecryptionKeyShare],
        config: &Self::ThresholdConfig,
    ) -> anyhow::Result<Self::DecryptionKey> {
        BIBEDecryptionKey::reconstruct(config, shares)
    }

    fn prepare_ct(
        ct: &Self::Ciphertext,
        digest: &Self::Digest,
        eval_proofs: &Self::EvalProofs,
    ) -> std::result::Result<Self::PreparedCiphertext, MissingEvalProofError> {
        ct.prepare(digest, eval_proofs)
    }

    fn decrypt<'a, P: Plaintext>(
        decryption_key: &Self::DecryptionKey,
        ct: &Self::PreparedCiphertext,
    ) -> anyhow::Result<P> {
        decryption_key.decrypt(ct)
    }

    fn verify_decryption_key_share(
        verification_key_share: &Self::VerificationKey,
        digest: &Self::Digest,
        decryption_key_share: &Self::DecryptionKeyShare,
    ) -> anyhow::Result<()> {
        verification_key_share.verify_decryption_key_share(digest, decryption_key_share)
    }

    fn verify_decryption_key(
        encryption_key: &Self::EncryptionKey,
        digest: &Self::Digest,
        decryption_key: &Self::DecryptionKey,
    ) -> Result<()> {
        encryption_key.verify_decryption_key(digest, decryption_key)
    }

    fn decrypt_slow<P: Plaintext>(
        decryption_key: &Self::DecryptionKey,
        ct: &Self::Ciphertext,
        digest: &Self::Digest,
        eval_proof: &Self::EvalProof,
    ) -> Result<P> {
        decryption_key.decrypt(&ct.prepare_individual(digest, eval_proof))
    }
}
