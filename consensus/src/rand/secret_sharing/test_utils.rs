// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_batch_encryption::{
    group::Fr, schemes::fptx_weighted::FPTXWeighted, traits::BatchThresholdEncryption,
};
use aptos_consensus_types::common::Author;
use aptos_crypto::{weighted_config::WeightedConfigArkworks, HashValue};
use aptos_types::{
    secret_sharing::{
        Digest, MasterSecretKeyShare, SecretShare, SecretShareConfig, SecretShareMetadata,
        SecretSharedKey,
    },
    validator_verifier::random_validator_verifier_with_voting_power,
};
use std::sync::Arc;

pub struct TestContext {
    pub authors: Vec<Author>,
    pub epoch: u64,
    pub secret_share_config: SecretShareConfig,
    pub msk_shares: Vec<MasterSecretKeyShare>,
}

impl TestContext {
    pub fn new(weights: Vec<u64>) -> Self {
        let num_validators = weights.len();
        let (signers, validator_verifier) =
            random_validator_verifier_with_voting_power(num_validators, None, false, &weights);
        let authors: Vec<Author> = signers.iter().map(|s| s.author()).collect();

        let total_weight: usize = weights.iter().map(|w| *w as usize).sum();
        let threshold = total_weight * 2 / 3 + 1;

        let tc = WeightedConfigArkworks::<Fr>::new(
            threshold,
            weights.iter().map(|w| *w as usize).collect(),
        )
        .expect("Failed to create weighted config");

        let (ek, dk, vks, msk_shares) =
            FPTXWeighted::setup_for_testing(8, 1, 1, &tc).expect("Failed to setup crypto");

        let secret_share_config = SecretShareConfig::new(
            Arc::new(validator_verifier),
            dk,
            msk_shares[0].clone(),
            vks,
            tc,
            ek,
        );

        Self {
            authors,
            epoch: 1,
            secret_share_config,
            msk_shares,
        }
    }
}

pub fn create_metadata(epoch: u64, round: u64) -> SecretShareMetadata {
    SecretShareMetadata::new(epoch, round, 0, HashValue::random(), Digest::default())
}

pub fn create_secret_share(
    ctx: &TestContext,
    author_index: usize,
    metadata: &SecretShareMetadata,
) -> SecretShare {
    let share =
        FPTXWeighted::derive_decryption_key_share(&ctx.msk_shares[author_index], &metadata.digest)
            .expect("Failed to derive key share");
    SecretShare::new(ctx.authors[author_index], metadata.clone(), share)
}

pub fn create_secret_shared_key(
    ctx: &TestContext,
    metadata: &SecretShareMetadata,
) -> SecretSharedKey {
    let shares: Vec<SecretShare> = (0..ctx.authors.len())
        .map(|i| create_secret_share(ctx, i, metadata))
        .collect();
    let key = SecretShare::aggregate(shares.iter(), &ctx.secret_share_config)
        .expect("Failed to aggregate");
    SecretSharedKey::new(metadata.clone(), key)
}
