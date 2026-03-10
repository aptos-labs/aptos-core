// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    errors::MissingEvalProofError,
    traits::{BatchThresholdEncryption, Plaintext},
};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

#[cfg(test)]
pub mod fptx_smoke;
#[cfg(test)]
pub mod fptx_succinct_smoke;
#[cfg(test)]
pub mod fptx_weighted_smoke;
#[cfg(test)]
pub mod typescript;

pub fn prepare_all<T: BatchThresholdEncryption, P: Plaintext>(
    cts: &[T::Ciphertext],
    digest: &T::Digest,
    eval_proofs: &T::EvalProofs,
) -> std::result::Result<Vec<T::PreparedCiphertext>, MissingEvalProofError> {
    cts.into_par_iter()
        .map(|ct| T::prepare_ct(ct, digest, eval_proofs))
        .collect::<std::result::Result<Vec<T::PreparedCiphertext>, MissingEvalProofError>>()
}

pub fn decrypt_all<T: BatchThresholdEncryption, P: Plaintext>(
    decryption_key: &T::DecryptionKey,
    cts: &[T::PreparedCiphertext],
) -> anyhow::Result<Vec<P>> {
    cts.into_par_iter()
        .map(|ct| {
            let plaintext: anyhow::Result<P> = T::decrypt(decryption_key, ct);
            plaintext
        })
        .collect::<anyhow::Result<Vec<P>>>()
}
