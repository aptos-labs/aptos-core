// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::random::random_scalar;
use velor_crypto::Uniform;
use velor_crypto_derive::{SilentDebug, SilentDisplay};
use blstrs::Scalar;
use ff::Field;
use num_traits::Zero;
use rand_core::{CryptoRng, RngCore};
use std::ops::{Add, AddAssign};

/// The *input secret* that will be given as input to the PVSS dealing algorithm. This will be of a
/// different type than the *dealt secret* that will be returned by the PVSS reconstruction algorithm.
///
/// This secret will NOT need to be stored by validators because a validator (1) picks such a secret
/// and (2) deals it via the PVSS. If the validator crashes during dealing, the entire task will be
/// restarted with a freshly-generated input secret.
#[derive(SilentDebug, SilentDisplay, PartialEq)]
pub struct InputSecret {
    /// The actual secret being dealt; a scalar $a \in F$.
    a: Scalar,
}

/// Make sure input secrets can be added together.
impl Add<&InputSecret> for InputSecret {
    type Output = InputSecret;

    fn add(self, rhs: &InputSecret) -> Self::Output {
        InputSecret {
            a: self.a.add(rhs.a),
        }
    }
}

impl AddAssign<&InputSecret> for InputSecret {
    fn add_assign(&mut self, rhs: &InputSecret) {
        self.a.add_assign(rhs.a)
    }
}

impl Add<InputSecret> for InputSecret {
    type Output = InputSecret;

    fn add(self, rhs: InputSecret) -> Self::Output {
        InputSecret {
            a: self.a.add(rhs.a),
        }
    }
}

impl Zero for InputSecret {
    fn zero() -> Self {
        InputSecret { a: Scalar::ZERO }
    }

    fn is_zero(&self) -> bool {
        self.a.is_zero_vartime()
    }
}

#[cfg(feature = "assert-private-keys-not-cloneable")]
static_assertions::assert_not_impl_any!(InputSecret: Clone);

impl InputSecret {
    pub fn get_secret_a(&self) -> &Scalar {
        &self.a
    }
}

impl Uniform for InputSecret {
    fn generate<R>(rng: &mut R) -> Self
    where
        R: RngCore + CryptoRng,
    {
        let a = random_scalar(rng);

        InputSecret { a }
    }
}
