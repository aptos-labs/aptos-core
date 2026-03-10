// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Minimal field trait for sumcheck (no Jolt dependency).
//! We use the field element F as the challenge type everywhere.

use ark_ff::Field;
use std::ops::{AddAssign, MulAssign};

/// Field type used in sumcheck. Challenge type = F.
pub trait SumcheckField: Field + Copy + Send + Sync + AddAssign + MulAssign {
    /// Challenge type; we use F itself.
    type Challenge: Copy + Send + Sync;

    /// Convert challenge to field element (identity when Challenge = F).
    fn challenge_to_field(c: &Self::Challenge) -> Self;

    /// Convert field element to challenge (identity when Challenge = F).
    fn into_challenge(f: Self) -> Self::Challenge;

    /// Returns self * 2^exp (for batching scaling).
    fn mul_pow_2(self, exp: usize) -> Self {
        let two = Self::from(2u64);
        let mut acc = Self::one();
        for _ in 0..exp {
            acc *= two;
        }
        self * acc
    }
}

impl<F> SumcheckField for F
where
    F: Field + Copy + Send + Sync + AddAssign + MulAssign,
{
    type Challenge = F;

    fn challenge_to_field(c: &Self::Challenge) -> Self {
        *c
    }

    fn into_challenge(f: Self) -> Self::Challenge {
        f
    }
}
