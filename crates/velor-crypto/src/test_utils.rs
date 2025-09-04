// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Internal module containing convenience utility functions mainly for testing

use crate::traits::Uniform;
use rand::distributions;
use serde::{Deserialize, Serialize};

/// A deterministic seed for PRNGs related to keys
pub const TEST_SEED: [u8; 32] = [0u8; 32];

/// A keypair consisting of a private and public key
#[cfg_attr(feature = "cloneable-private-keys", derive(Clone))]
#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyPair<S, P>
where
    for<'a> P: From<&'a S>,
{
    /// the private key component
    pub private_key: S,
    /// the public key component
    pub public_key: P,
}

impl<S, P> From<S> for KeyPair<S, P>
where
    for<'a> P: From<&'a S>,
{
    fn from(private_key: S) -> Self {
        KeyPair {
            public_key: (&private_key).into(),
            private_key,
        }
    }
}

impl<S, P> Uniform for KeyPair<S, P>
where
    S: Uniform,
    for<'a> P: From<&'a S>,
{
    fn generate<R>(rng: &mut R) -> Self
    where
        R: ::rand::RngCore + ::rand::CryptoRng,
    {
        let private_key = S::generate(rng);
        private_key.into()
    }
}

/// A pair consisting of a private and public key
impl<S, P> Uniform for (S, P)
where
    S: Uniform,
    for<'a> P: From<&'a S>,
{
    fn generate<R>(rng: &mut R) -> Self
    where
        R: ::rand::RngCore + ::rand::CryptoRng,
    {
        let private_key = S::generate(rng);
        let public_key = (&private_key).into();
        (private_key, public_key)
    }
}

impl<Priv, Pub> std::fmt::Debug for KeyPair<Priv, Pub>
where
    Priv: Serialize,
    Pub: Serialize + for<'a> From<&'a Priv>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut v = bcs::to_bytes(&self.private_key).unwrap();
        v.extend(&bcs::to_bytes(&self.public_key).unwrap());
        write!(f, "{}", hex::encode(&v[..]))
    }
}

#[cfg(any(test, feature = "fuzzing"))]
use crate::signing_message;
#[cfg(any(test, feature = "fuzzing"))]
use curve25519_dalek::constants::EIGHT_TORSION;
#[cfg(any(test, feature = "fuzzing"))]
use curve25519_dalek::edwards::EdwardsPoint;
#[cfg(any(test, feature = "fuzzing"))]
use curve25519_dalek::scalar::Scalar;
#[cfg(any(test, feature = "fuzzing"))]
use curve25519_dalek::traits::Identity;
#[cfg(any(test, feature = "fuzzing"))]
use digest::Digest;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;
use rand::prelude::IteratorRandom;
#[cfg(any(test, feature = "fuzzing"))]
use rand::{rngs::StdRng, SeedableRng};
#[cfg(any(test, feature = "fuzzing"))]
use sha2::Sha512;

/// Produces a uniformly random keypair from a seed
#[cfg(any(test, feature = "fuzzing"))]
pub fn uniform_keypair_strategy<Priv, Pub>() -> impl Strategy<Value = KeyPair<Priv, Pub>>
where
    Pub: Serialize + for<'a> From<&'a Priv>,
    Priv: Serialize + Uniform,
{
    // The no_shrink is because keypairs should be fixed -- shrinking would cause a different
    // keypair to be generated, which appears to not be very useful.
    any::<[u8; 32]>()
        .prop_map(|seed| {
            let mut rng = StdRng::from_seed(seed);
            KeyPair::<Priv, Pub>::generate(&mut rng)
        })
        .no_shrink()
}

/// Produces a small order group element
#[cfg(any(test, feature = "fuzzing"))]
pub fn small_order_strategy() -> impl Strategy<Value = EdwardsPoint> {
    (0..EIGHT_TORSION.len())
        .prop_map(|exp| {
            let generator = EIGHT_TORSION[1]; // generator of size-8 subgroup is at index 1
            Scalar::from(exp as u64) * generator
        })
        .no_shrink()
}

/// Produces a small order R, public key A and a hash h = H(R, A, m) such that sB - hA = R when s is
/// zero.
#[allow(non_snake_case)]
#[cfg(any(test, feature = "fuzzing"))]
pub fn small_order_pk_with_adversarial_message(
) -> impl Strategy<Value = (EdwardsPoint, EdwardsPoint, TestVelorCrypto)> {
    (
        small_order_strategy(),
        small_order_strategy(),
        random_serializable_struct(),
    )
        .prop_filter(
            "Filtering messages by hash * pk == R",
            |(R, pk_point, msg)| {
                let pk_bytes = pk_point.compress().to_bytes();

                let msg_bytes = signing_message(msg).unwrap();

                let mut h: Sha512 = Sha512::new();
                h.update(R.compress().as_bytes());
                h.update(pk_bytes);
                h.update(msg_bytes);

                let k = Scalar::from_hash(h);

                k * pk_point + (*R) == EdwardsPoint::identity()
            },
        )
}

/// Produces a uniformly random keypair from a seed and the user can alter this sleed slightly.
/// Useful for circumstances where you want two disjoint keypair generations that may interact with
/// each other.
#[cfg(any(test, feature = "fuzzing"))]
pub fn uniform_keypair_strategy_with_perturbation<Priv, Pub>(
    perturbation: u8,
) -> impl Strategy<Value = KeyPair<Priv, Pub>>
where
    Pub: Serialize + for<'a> From<&'a Priv>,
    Priv: Serialize + Uniform,
{
    // The no_shrink is because keypairs should be fixed -- shrinking would cause a different
    // keypair to be generated, which appears to not be very useful.
    any::<[u8; 32]>()
        .prop_map(move |mut seed| {
            for elem in seed.iter_mut() {
                *elem = elem.saturating_add(perturbation);
            }
            let mut rng = StdRng::from_seed(seed);
            KeyPair::<Priv, Pub>::generate(&mut rng)
        })
        .no_shrink()
}

/// Returns `subset_size` numbers picked uniformly at random from 0 to `max_set_size - 1` (inclusive).
pub fn random_subset<R>(mut rng: &mut R, max_set_size: usize, subset_size: usize) -> Vec<usize>
where
    R: ::rand::Rng + ?Sized,
{
    let mut vec = (0..max_set_size)
        .choose_multiple(&mut rng, subset_size)
        .into_iter()
        .collect::<Vec<usize>>();

    vec.sort_unstable();

    vec
}

/// Returns n random bytes.
pub fn random_bytes<R>(rng: &mut R, n: usize) -> Vec<u8>
where
    R: ::rand::Rng + Copy,
{
    let range = distributions::Uniform::from(0u8..u8::MAX);
    rng.sample_iter(&range).take(n).collect()
}

/// Generates `num_signers` random key-pairs.
pub fn random_keypairs<R, PrivKey, PubKey>(
    mut rng: &mut R,
    num_signers: usize,
) -> Vec<KeyPair<PrivKey, PubKey>>
where
    R: ::rand::RngCore + ::rand::CryptoRng,
    PubKey: for<'a> std::convert::From<&'a PrivKey>,
    PrivKey: Uniform,
{
    let mut key_pairs = vec![];
    for _ in 0..num_signers {
        key_pairs.push(KeyPair::<PrivKey, PubKey>::generate(&mut rng));
    }
    key_pairs
}

/// This struct provides a means of testing signing and verification through
/// BCS serialization and domain separation
//#[cfg(any(test, feature = "fuzzing"))]
#[derive(Debug, Serialize, Deserialize)]
pub struct TestVelorCrypto(pub String);

// the following block is macro expanded from derive(CryptoHasher, BCSCryptoHash)

/// Cryptographic hasher for an BCS-serializable #item
// #[cfg(any(test, feature = "fuzzing"))]
pub struct TestVelorCryptoHasher(crate::hash::DefaultHasher);
// #[cfg(any(test, feature = "fuzzing"))]
impl ::core::clone::Clone for TestVelorCryptoHasher {
    #[inline]
    fn clone(&self) -> TestVelorCryptoHasher {
        match *self {
            TestVelorCryptoHasher(ref __self_0_0) => {
                TestVelorCryptoHasher(::core::clone::Clone::clone(__self_0_0))
            },
        }
    }
}
// #[cfg(any(test, feature = "fuzzing"))]
static TEST_CRYPTO_SEED: crate::_once_cell::sync::OnceCell<[u8; 32]> =
    crate::_once_cell::sync::OnceCell::new();
// #[cfg(any(test, feature = "fuzzing"))]
impl TestVelorCryptoHasher {
    fn new() -> Self {
        let name = crate::_serde_name::trace_name::<TestVelorCrypto>()
            .expect("The `CryptoHasher` macro only applies to structs and enums");
        TestVelorCryptoHasher(crate::hash::DefaultHasher::new(name.as_bytes()))
    }
}
// #[cfg(any(test, feature = "fuzzing"))]
static TEST_CRYPTO_HASHER: crate::_once_cell::sync::Lazy<TestVelorCryptoHasher> =
    crate::_once_cell::sync::Lazy::new(TestVelorCryptoHasher::new);
// #[cfg(any(test, feature = "fuzzing"))]
impl std::default::Default for TestVelorCryptoHasher {
    fn default() -> Self {
        TEST_CRYPTO_HASHER.clone()
    }
}
// #[cfg(any(test, feature = "fuzzing"))]
impl crate::hash::CryptoHasher for TestVelorCryptoHasher {
    fn seed() -> &'static [u8; 32] {
        TEST_CRYPTO_SEED.get_or_init(|| {
            let name = crate::_serde_name::trace_name::<TestVelorCrypto>()
                .expect("The `CryptoHasher` macro only applies to structs and enums.")
                .as_bytes();
            crate::hash::DefaultHasher::prefixed_hash(name)
        })
    }

    fn update(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    fn finish(self) -> crate::hash::HashValue {
        self.0.finish()
    }
}
// #[cfg(any(test, feature = "fuzzing"))]
impl std::io::Write for TestVelorCryptoHasher {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
// #[cfg(any(test, feature = "fuzzing"))]
impl crate::hash::CryptoHash for TestVelorCrypto {
    type Hasher = TestVelorCryptoHasher;

    fn hash(&self) -> crate::hash::HashValue {
        use crate::hash::CryptoHasher;
        let mut state = Self::Hasher::default();
        bcs::serialize_into(&mut state, &self)
            .expect("BCS serialization of TestVelorCrypto should not fail");
        state.finish()
    }
}

/// Produces a random TestVelorCrypto signable / verifiable struct.
#[cfg(any(test, feature = "fuzzing"))]
pub fn random_serializable_struct() -> impl Strategy<Value = TestVelorCrypto> {
    (String::arbitrary()).prop_map(TestVelorCrypto).no_shrink()
}
