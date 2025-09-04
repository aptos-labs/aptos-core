// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    algebra::lagrange::lagrange_coefficients,
    pvss::{
        traits::{Reconstructable, SecretSharingConfig},
        Player, ThresholdConfig,
    },
};
use blstrs::Scalar;
use ff::Field;
use more_asserts::{assert_ge, assert_le};

impl Reconstructable<ThresholdConfig> for Scalar {
    type Share = Scalar;

    fn reconstruct(sc: &ThresholdConfig, shares: &Vec<(Player, Self::Share)>) -> Self {
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

        assert_eq!(lagr.len(), shares.len());

        shares
            .iter()
            .zip(lagr.iter())
            .map(|(&share, &lagr)| share * lagr)
            .sum::<Scalar>()
    }
}
