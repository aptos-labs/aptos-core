// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::account_address::AccountAddress;
use velor_crypto::{
    bls12381, hash::CryptoHash, test_utils::TEST_SEED, CryptoMaterialError, PrivateKey, SigningKey,
    Uniform,
};
use rand::{rngs::StdRng, SeedableRng};
use serde::ser::Serialize;
use std::{convert::TryFrom, sync::Arc};

/// ValidatorSigner associates an author with public and private keys with helpers for signing and
/// validating. This struct can be used for all signing operations including block and network
/// signing, respectively.
#[derive(Debug)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Clone))]
pub struct ValidatorSigner {
    author: AccountAddress,
    private_key: Arc<bls12381::PrivateKey>,
}

impl ValidatorSigner {
    pub fn new(author: AccountAddress, private_key: Arc<bls12381::PrivateKey>) -> Self {
        ValidatorSigner {
            author,
            private_key,
        }
    }

    /// Constructs a signature for `message` using `private_key`.
    pub fn sign<T: Serialize + CryptoHash>(
        &self,
        message: &T,
    ) -> Result<bls12381::Signature, CryptoMaterialError> {
        self.private_key.sign(message)
    }

    /// Returns the author associated with this signer.
    pub fn author(&self) -> AccountAddress {
        self.author
    }

    /// Returns the public key associated with this signer.
    pub fn public_key(&self) -> bls12381::PublicKey {
        self.private_key.public_key()
    }

    /// Returns the private key associated with this signer. Only available for testing purposes.
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn private_key(&self) -> &bls12381::PrivateKey {
        self.private_key.as_ref()
    }
}

impl ValidatorSigner {
    /// Generate a random set of public and private keys and author
    /// information.
    /// This takes an optional seed, which it initializes to
    /// `test_utils::TEST_SEED` if passed `None`
    pub fn random(opt_rng_seed: impl for<'a> Into<Option<[u8; 32]>>) -> Self {
        let mut rng = StdRng::from_seed(opt_rng_seed.into().unwrap_or(TEST_SEED));
        Self::new(
            AccountAddress::random(),
            Arc::new(bls12381::PrivateKey::generate(&mut rng)),
        )
    }

    /// For test only - makes signer with nicely looking account address that has specified integer
    /// as fist byte, and rest are zeroes
    pub fn from_int(num: u8) -> Self {
        let mut address = [0; AccountAddress::LENGTH];
        address[0] = num;
        let private_key = bls12381::PrivateKey::generate_for_testing();
        Self::new(
            AccountAddress::try_from(&address[..]).unwrap(),
            Arc::new(private_key),
        )
    }
}

#[cfg(any(test, feature = "fuzzing"))]
pub mod proptests {
    use super::*;
    use velor_crypto::Genesis;
    use proptest::{prelude::*, sample, strategy::LazyJust};

    #[allow(clippy::redundant_closure)]
    pub fn arb_signing_key() -> impl Strategy<Value = bls12381::PrivateKey> {
        prop_oneof![
            // The no_shrink here reflects that particular keypair choices out
            // of random options are irrelevant.
            LazyJust::new(|| bls12381::PrivateKey::generate_for_testing()),
            LazyJust::new(|| bls12381::PrivateKey::genesis()),
        ]
    }

    pub fn signer_strategy(
        signing_key_strategy: impl Strategy<Value = bls12381::PrivateKey>,
    ) -> impl Strategy<Value = ValidatorSigner> {
        signing_key_strategy.prop_map(|signing_key| {
            ValidatorSigner::new(
                AccountAddress::from_bytes(&signing_key.public_key().to_bytes()[0..32]).unwrap(),
                Arc::new(signing_key),
            )
        })
    }

    #[allow(clippy::redundant_closure)]
    pub fn rand_signer() -> impl Strategy<Value = ValidatorSigner> {
        signer_strategy(arb_signing_key())
    }

    #[allow(clippy::redundant_closure)]
    pub fn arb_signer() -> impl Strategy<Value = ValidatorSigner> {
        prop_oneof![
            rand_signer(),
            LazyJust::new(|| {
                let genesis_key = bls12381::PrivateKey::genesis();
                ValidatorSigner::new(AccountAddress::random(), Arc::new(genesis_key))
            })
        ]
    }

    fn select_keypair(
        keys: Vec<bls12381::PrivateKey>,
    ) -> impl Strategy<Value = bls12381::PrivateKey> {
        sample::select(keys)
    }

    pub fn mostly_in_keypair_pool(
        keys: Vec<bls12381::PrivateKey>,
    ) -> impl Strategy<Value = ValidatorSigner> {
        prop::strategy::Union::new_weighted(vec![
            (9, signer_strategy(select_keypair(keys)).boxed()),
            (1, arb_signer().boxed()),
        ])
    }

    proptest! {
        #[test]
        fn test_new_signer(signing_key in arb_signing_key()){
            let public_key = signing_key.public_key();
            let signer = ValidatorSigner::new(AccountAddress::random(), Arc::new(signing_key));
            prop_assert_eq!(public_key, signer.public_key());
        }

    }
}
