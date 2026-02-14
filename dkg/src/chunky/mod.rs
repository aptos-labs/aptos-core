// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_batch_encryption::shared::digest::DigestKey;
use once_cell::sync::Lazy;

pub mod agg_subtrx_producer;
pub mod dkg_manager;
pub mod missing_transcript_fetcher;
pub mod subtrx_cert_producer;
pub mod types;

/// Shared test DigestKey for encryption key derivation.
/// TODO(ibalajiarun): Replace with proper trusted setup for production.
pub static TEST_DIGEST_KEY: Lazy<DigestKey> = Lazy::new(|| {
    use ark_std::rand::SeedableRng;
    let mut rng = ark_std::rand::rngs::StdRng::seed_from_u64(100u64);
    DigestKey::new(&mut rng, 32, 200).expect("DigestKey creation should not fail")
});
