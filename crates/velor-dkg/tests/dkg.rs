// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_crypto::hash::CryptoHash;
use velor_dkg::{
    pvss::{
        das,
        das::unweighted_protocol,
        insecure_field, test_utils,
        test_utils::{reconstruct_dealt_secret_key_randomly, NoAux},
        traits::{SecretSharingConfig, Transcript},
        weighted::generic_weighting::GenericWeighting,
    },
    utils::random::random_scalar,
};
use rand::{rngs::StdRng, thread_rng};
use rand_core::SeedableRng;

#[test]
fn test_dkg_all_unweighted() {
    let mut rng = thread_rng();
    let tcs = test_utils::get_threshold_configs_for_testing();
    let seed = random_scalar(&mut rng);

    aggregatable_dkg::<unweighted_protocol::Transcript>(tcs.last().unwrap(), seed.to_bytes_le());
    aggregatable_dkg::<insecure_field::Transcript>(tcs.last().unwrap(), seed.to_bytes_le());
}

#[test]
fn test_dkg_all_weighted() {
    let mut rng = thread_rng();
    let wcs = test_utils::get_weighted_configs_for_testing();
    let seed = random_scalar(&mut rng);

    aggregatable_dkg::<GenericWeighting<unweighted_protocol::Transcript>>(
        wcs.last().unwrap(),
        seed.to_bytes_le(),
    );
    aggregatable_dkg::<GenericWeighting<das::Transcript>>(wcs.last().unwrap(), seed.to_bytes_le());
    aggregatable_dkg::<das::WeightedTranscript>(wcs.last().unwrap(), seed.to_bytes_le());
}

/// Deals `n` times, aggregates all transcripts, and attempts to reconstruct the secret dealt in this
/// aggregated transcript.
fn aggregatable_dkg<T: Transcript + CryptoHash>(sc: &T::SecretSharingConfig, seed_bytes: [u8; 32]) {
    let mut rng = StdRng::from_seed(seed_bytes);

    let d = test_utils::setup_dealing::<T, StdRng>(sc, &mut rng);

    let mut trxs = vec![];

    // Deal `n` transcripts
    for i in 0..sc.get_total_num_players() {
        trxs.push(T::deal(
            sc,
            &d.pp,
            &d.ssks[i],
            &d.eks,
            &d.iss[i],
            &NoAux,
            &sc.get_player(i),
            &mut rng,
        ));
    }

    // Aggregate all `n` transcripts
    let trx = T::aggregate(sc, trxs).unwrap();

    // Verify the aggregated transcript
    trx.verify(
        sc,
        &d.pp,
        &d.spks,
        &d.eks,
        &(0..sc.get_total_num_players())
            .map(|_| NoAux)
            .collect::<Vec<NoAux>>(),
    )
    .expect("aggregated PVSS transcript failed verification");

    if d.dsk != reconstruct_dealt_secret_key_randomly::<StdRng, T>(sc, &mut rng, &d.dks, trx) {
        panic!("Reconstructed SK did not match");
    }
}
