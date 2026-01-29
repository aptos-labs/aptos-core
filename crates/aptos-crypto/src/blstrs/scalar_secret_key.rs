// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Implements unweighted threshold secret reconstruction for BLSTRS scalars.

use crate::{
    arkworks::shamir::{Reconstructable, ShamirShare},
    blstrs::{lagrange::lagrange_coefficients, threshold_config::ThresholdConfigBlstrs},
    traits::{TSecretSharingConfig as _, ThresholdConfig as _},
};
use blstrs::Scalar;
use ff::Field;
use more_asserts::{assert_ge, assert_le};

impl Reconstructable<ThresholdConfigBlstrs> for Scalar {
    type ShareValue = Scalar;

    fn reconstruct(
        sc: &ThresholdConfigBlstrs,
        shares: &[ShamirShare<Self::ShareValue>],
    ) -> anyhow::Result<Self> {
        assert_ge!(shares.len(), sc.get_threshold());
        assert_le!(shares.len(), sc.get_total_num_players());

        let ids = shares.iter().map(|(p, _)| p.id).collect::<Vec<usize>>();
        let lagr = lagrange_coefficients(
            sc.get_batch_evaluation_domain(),
            ids.as_slice(),
            &Scalar::ZERO,
        );
        let shares = shares
            .iter()
            .map(|(_, share)| *share)
            .collect::<Vec<Scalar>>();

        // TODO should this return a
        assert_eq!(lagr.len(), shares.len());

        Ok(shares
            .iter()
            .zip(lagr.iter())
            .map(|(&share, &lagr)| share * lagr)
            .sum::<Scalar>())
    }
}
