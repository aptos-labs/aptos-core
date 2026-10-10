// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::{weighted_config::WeightedConfigArkworks, SigningKey, Uniform};
use aptos_dkg::pvss::{
    chunky,
    test_utils::{
        reconstruct_dealt_secret_key_randomly, setup_dealing_chunky_all_four_with_pp, DealingArgs,
    },
    traits::transcript::{HasAggregatableSubtranscript, Transcript, WithMaxNumShares},
    Player,
};
use ark_bls12_381::{Bls12_381, Fr};
use rand::{rngs::StdRng, SeedableRng};

#[test]
fn test_chunky_dealer_and_recipient_sets() {
    let mut rng = StdRng::seed_from_u64(206);
    let config = WeightedConfigArkworks::<Fr>::new(3, vec![1, 2, 1]).unwrap();
    let pp = chunky::PublicParameters::<Bls12_381>::with_max_num_shares(4);

    // Cover shrinking, growing, and equally sized sets. Dealer signing keys are
    // generated independently of the recipient encryption keys in every case.
    for num_dealers in [4, 2, 3] {
        let (unsigned_v1, unsigned_v2, signed_v1, signed_v2) =
            setup_dealing_chunky_all_four_with_pp(&config, &pp, &mut rng);
        check_dealer_and_recipient_sets(&config, unsigned_v1, num_dealers, &mut rng);
        check_dealer_and_recipient_sets(&config, unsigned_v2, num_dealers, &mut rng);
        check_dealer_and_recipient_sets(&config, signed_v1, num_dealers, &mut rng);
        check_dealer_and_recipient_sets(&config, signed_v2, num_dealers, &mut rng);
    }
}

fn check_dealer_and_recipient_sets<T>(
    config: &WeightedConfigArkworks<Fr>,
    mut args: DealingArgs<T>,
    num_dealers: usize,
    rng: &mut StdRng,
) where
    T: HasAggregatableSubtranscript + Transcript<SecretSharingConfig = WeightedConfigArkworks<Fr>>,
{
    args.ssks = (0..num_dealers)
        .map(|_| T::SigningSecretKey::generate(rng))
        .collect();
    args.spks = args.ssks.iter().map(SigningKey::verifying_key).collect();

    // In the shrinking case this valid dealer index is outside the recipient
    // set. Dealer identities must be interpreted in the signing-key list.
    let dealer = Player {
        id: num_dealers - 1,
    };
    let session_id = 17548u64;
    let transcript = T::deal(
        config,
        &args.pp,
        &args.ssks[dealer.id],
        &args.spks[dealer.id],
        &args.eks,
        &args.s,
        &session_id,
        &dealer,
        rng,
    );
    transcript
        .verify(config, &args.pp, &args.spks, &args.eks, &session_id, rng)
        .unwrap_or_else(|err| {
            panic!(
                "{} with {num_dealers} dealers and {} recipients failed: {err:#}",
                T::scheme_name(),
                args.eks.len(),
            )
        });

    // The dealer's index must still be in bounds for the dealer key list.
    assert!(transcript
        .verify(
            config,
            &args.pp,
            &args.spks[..dealer.id],
            &args.eks,
            &session_id,
            rng,
        )
        .is_err());

    // Reordering dealer keys must not authenticate the original dealer.
    let mut wrong_signing_keys = args.spks.clone();
    wrong_signing_keys.swap(0, dealer.id);
    assert!(transcript
        .verify(
            config,
            &args.pp,
            &wrong_signing_keys,
            &args.eks,
            &session_id,
            rng,
        )
        .is_err());

    // Recipient keys must still match the recipient configuration.
    assert!(transcript
        .verify(
            config,
            &args.pp,
            &args.spks,
            &args.eks[..args.eks.len() - 1],
            &session_id,
            rng,
        )
        .is_err());

    // Proofs and signatures remain bound to their original session.
    assert!(transcript
        .verify(
            config,
            &args.pp,
            &args.spks,
            &args.eks,
            &(session_id + 1),
            rng
        )
        .is_err());

    assert!(
        args.dsk
            == reconstruct_dealt_secret_key_randomly::<_, T>(
                config, rng, &args.dks, transcript, &args.pp,
            )
    );
}
