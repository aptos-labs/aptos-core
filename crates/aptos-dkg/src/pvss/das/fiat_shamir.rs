// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    fiat_shamir,
    pvss::{traits, ThresholdConfig},
};
use serde::Serialize;

/// Securely derives a Fiat-Shamir challenge via Merlin.
/// Returns (n+1-t) random scalars for the SCRAPE LDT test (i.e., the random polynomial itself).
/// Additionally returns `num_scalars` random scalars for some linear combinations.
pub(crate) fn derive_challenge_scalars<T: traits::Transcript, A: Serialize>(
    trx: &T,
    sc: &ThresholdConfig,
    pp: &T::PublicParameters,
    spks: &Vec<T::SigningPubKey>,
    eks: &Vec<T::EncryptPubKey>,
    auxs: &Vec<A>,
    dst: &[u8],
    num_scalars: usize,
) -> (Vec<blstrs::Scalar>, Vec<blstrs::Scalar>) {
    let mut fs_t = fiat_shamir::initialize_pvss_transcript::<T>(sc, pp, eks, dst);

    <merlin::Transcript as fiat_shamir::PVSS<T>>::append_signing_pub_keys(&mut fs_t, spks);
    <merlin::Transcript as fiat_shamir::PVSS<T>>::append_auxs(&mut fs_t, auxs);
    <merlin::Transcript as fiat_shamir::PVSS<T>>::append_transcript(&mut fs_t, trx);

    (
        <merlin::Transcript as fiat_shamir::PVSS<T>>::challenge_dual_code_word_polynomial(
            &mut fs_t,
            sc.t,
            sc.n + 1,
        ),
        <merlin::Transcript as fiat_shamir::PVSS<T>>::challenge_linear_combination_scalars(
            &mut fs_t,
            num_scalars,
        ),
    )
}
