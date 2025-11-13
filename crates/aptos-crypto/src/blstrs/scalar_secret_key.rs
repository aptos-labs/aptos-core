// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::traits::{SecretSharingConfig as _, ThresholdConfig as _};
use crate::arkworks::shamir::Reconstructable;
use crate::{player::Player, blstrs::threshold_config::ThresholdConfigBlstrs};
use crate::blstrs::lagrange::lagrange_coefficients;
use blstrs::Scalar;
use ff::Field;
use more_asserts::{assert_ge, assert_le};

impl Reconstructable<ThresholdConfigBlstrs> for Scalar {
    type ShareValue = Scalar;

    fn reconstruct(sc: &ThresholdConfigBlstrs, shares: &Vec<(Player, Self::ShareValue)>) -> anyhow::Result<Self> {
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
