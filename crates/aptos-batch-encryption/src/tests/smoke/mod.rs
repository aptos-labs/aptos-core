// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    errors::MissingEvalProofError,
    traits::{BatchThresholdEncryption, DecryptionKeyShare},
};
use anyhow::Result;
use aptos_crypto::{player::Player, TSecretSharingConfig};
use rayon::iter::{
    IndexedParallelIterator as _, IntoParallelIterator, IntoParallelRefIterator as _,
    ParallelIterator,
};
use std::collections::HashMap;

#[cfg(test)]
pub mod fptx_smoke;
#[cfg(test)]
pub mod fptx_succinct_smoke;
pub mod fptx_weighted_smoke;

// b/c Strings can't be const, and only String implements Plaintext not &str
fn plaintext() -> String {
    String::from("test plaintext")
}
fn associated_data() -> String {
    String::from("test associated data")
}

pub struct SmokeTest<Scheme: BatchThresholdEncryption> {
    tc: <Scheme as BatchThresholdEncryption>::ThresholdConfig,
    ek: <Scheme as BatchThresholdEncryption>::EncryptionKey,
    dk: <Scheme as BatchThresholdEncryption>::DigestKey,
    vks: Vec<<Scheme as BatchThresholdEncryption>::VerificationKey>,
    msk_shares: Vec<<Scheme as BatchThresholdEncryption>::MasterSecretKeyShare>,
}

pub struct Decryption<Scheme: BatchThresholdEncryption> {
    dec_key: Scheme::DecryptionKey,
    digest: Scheme::Digest,
    eval_proofs: Scheme::EvalProofs,
    cts: Vec<Scheme::Ciphertext>,
    plaintexts: Vec<String>,
}

impl<Scheme: BatchThresholdEncryption> Decryption<Scheme> {
    pub fn test_decryption_verification(&self) {
        for ct in &self.cts {
            let eval_proof = Scheme::eval_proof_for_ct(&self.eval_proofs, ct).unwrap();
            let individual_decrypted_plaintext: String =
                Scheme::decrypt_slow(&self.dec_key, ct, &self.digest, &eval_proof).unwrap();
            assert_eq!(individual_decrypted_plaintext, plaintext());
        }
    }
}

impl<Scheme: BatchThresholdEncryption> SmokeTest<Scheme> {
    pub fn new(
        tc: <Scheme as BatchThresholdEncryption>::ThresholdConfig,
        ek: <Scheme as BatchThresholdEncryption>::EncryptionKey,
        dk: <Scheme as BatchThresholdEncryption>::DigestKey,
        vks: Vec<<Scheme as BatchThresholdEncryption>::VerificationKey>,
        msk_shares: Vec<<Scheme as BatchThresholdEncryption>::MasterSecretKeyShare>,
    ) -> Self {
        Self {
            tc,
            ek,
            dk,
            vks,
            msk_shares,
        }
    }

    pub fn do_decryption(&self, round: u64, cts: Vec<Scheme::Ciphertext>) -> Decryption<Scheme> {
        let mut rng_aptos = rand::thread_rng();

        let (d, pfs_promise) = Scheme::digest(&self.dk, &cts, round).unwrap();
        let pfs = Scheme::eval_proofs_compute_all(&pfs_promise, &self.dk);

        let dk_shares: Vec<<Scheme as BatchThresholdEncryption>::DecryptionKeyShare> = self
            .msk_shares
            .par_iter()
            .map(|msk_share| {
                <Scheme as BatchThresholdEncryption>::derive_decryption_key_share(msk_share, &d)
                    .unwrap()
            })
            .collect();

        dk_shares
            .par_iter()
            .zip(&self.vks)
            .map(|(dk_share, vk)| Scheme::verify_decryption_key_share(vk, &d, dk_share))
            .collect::<Result<Vec<()>>>()
            .unwrap();

        let dk_shares_map: HashMap<Player, Scheme::DecryptionKeyShare> = HashMap::from_iter(
            dk_shares
                .into_iter()
                .map(|dk_share| (dk_share.player(), dk_share)),
        );

        let eligible_share_subset: Vec<<Scheme as BatchThresholdEncryption>::DecryptionKeyShare> =
            self.tc
                .get_random_eligible_subset_of_players(&mut rng_aptos)
                .into_par_iter()
                .map(|player| dk_shares_map[&player].clone())
                .collect();

        let dec_key = Scheme::reconstruct_decryption_key(&eligible_share_subset, &self.tc).unwrap();

        <Scheme as BatchThresholdEncryption>::verify_decryption_key(&self.ek, &d, &dec_key)
            .unwrap();

        let prepared_cts = cts
            .par_iter()
            .map(|ct| <Scheme as BatchThresholdEncryption>::prepare_ct(ct, &d, &pfs))
            .collect::<Result<Vec<Scheme::PreparedCiphertext>, MissingEvalProofError>>()
            .unwrap();

        let plaintexts = prepared_cts
            .into_par_iter()
            .map(|prepared_ct| Scheme::decrypt(&dec_key, &prepared_ct))
            .collect::<Result<Vec<String>>>()
            .unwrap();

        Decryption {
            dec_key,
            digest: d,
            eval_proofs: pfs,
            cts,
            plaintexts,
        }
    }

    pub fn run_with_one_ct(&self, round: u64) -> Decryption<Scheme> {
        let mut rng_arkworks = ark_std::rand::thread_rng();

        let ct = Scheme::encrypt(
            &self.ek,
            &mut rng_arkworks,
            &plaintext(),
            &associated_data(),
        )
        .unwrap();
        Scheme::verify_ct(&ct, &associated_data()).unwrap();

        let result = self.do_decryption(round, vec![ct]);

        assert_eq!(result.plaintexts[0], plaintext());

        result
    }

    pub fn run_with_max_cts(&self, round: u64) -> Decryption<Scheme> {
        let cts = (0..Scheme::max_batch_size(&self.dk))
            .into_par_iter()
            .map(|_| {
                let mut rng_arkworks = ark_std::rand::thread_rng();
                Scheme::encrypt(
                    &self.ek,
                    &mut rng_arkworks,
                    &plaintext(),
                    &associated_data(),
                )
                .unwrap()
            })
            .collect::<Vec<Scheme::Ciphertext>>();

        let result = self.do_decryption(round, cts);

        (0..result.cts.len()).into_par_iter().for_each(|i| {
            Scheme::verify_ct(&result.cts[i], &associated_data()).unwrap();
            assert_eq!(result.plaintexts[i], plaintext());
        });

        result
    }
}
