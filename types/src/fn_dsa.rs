// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! FN-DSA-512 (Falcon-512) keys and signatures for transactions. EXPERIMENT ONLY.
//!
//! FIPS 206 is not published yet, and the backing C implementation is not conformant.
//! These types exist to measure the cost of post-quantum transaction signatures.

use anyhow::{ensure, Result};
use aptos_crypto::{
    hash::{CryptoHash, HashValue},
    traits::signing_message,
};
use aptos_fn_dsa::{
    keygen_from_seed, verify, ExpandedSigningKey, SIGNATURE_LENGTH, SIGN_SEED_LENGTH,
    VERIFYING_KEY_LENGTH,
};
use serde::{
    de::{self, Deserializer},
    ser::Serializer,
    Deserialize, Serialize,
};
use std::{fmt, sync::Arc};

const SIGN_SEED_DOMAIN: &[u8] = b"APTOS_FN_DSA_512_TXN_SIGN_SEED";

macro_rules! fixed_bytes_type {
    ($name:ident, $length:expr, $serde_name:literal) => {
        #[derive(Clone, PartialEq, Eq, Hash)]
        pub struct $name(Arc<[u8; $length]>);

        impl $name {
            pub const LENGTH: usize = $length;

            pub fn as_bytes(&self) -> &[u8] {
                &self.0[..]
            }

            pub fn to_bytes(&self) -> Vec<u8> {
                self.0.to_vec()
            }
        }

        impl TryFrom<&[u8]> for $name {
            type Error = anyhow::Error;

            fn try_from(bytes: &[u8]) -> Result<Self> {
                ensure!(
                    bytes.len() == $length,
                    "{}: wrong length {} (expected {})",
                    $serde_name,
                    bytes.len(),
                    $length
                );
                let mut array = [0u8; $length];
                array.copy_from_slice(bytes);
                Ok(Self(Arc::new(array)))
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "0x{}", hex::encode(&self.0[..]))
            }
        }

        // Shows a short prefix only.
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "{}(0x{}..)", $serde_name, hex::encode(&self.0[..8]))
            }
        }

        impl Serialize for $name {
            fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                if serializer.is_human_readable() {
                    serializer.serialize_str(&self.to_string())
                } else {
                    serializer
                        .serialize_newtype_struct($serde_name, serde_bytes::Bytes::new(&self.0[..]))
                }
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                if deserializer.is_human_readable() {
                    let encoded = <String>::deserialize(deserializer)?;
                    let stripped = encoded.strip_prefix("0x").unwrap_or(&encoded);
                    let bytes = hex::decode(stripped).map_err(de::Error::custom)?;
                    Self::try_from(bytes.as_slice()).map_err(de::Error::custom)
                } else {
                    #[derive(Deserialize)]
                    #[serde(rename = $serde_name)]
                    struct Value<'a>(&'a [u8]);

                    let value = Value::deserialize(deserializer)?;
                    Self::try_from(value.0).map_err(de::Error::custom)
                }
            }
        }
    };
}

fixed_bytes_type!(FnDsa512PublicKey, VERIFYING_KEY_LENGTH, "FnDsa512PublicKey");
fixed_bytes_type!(FnDsa512Signature, SIGNATURE_LENGTH, "FnDsa512Signature");

impl FnDsa512Signature {
    /// Verifies the signature on the signing message of `message`, as for other schemes.
    pub fn verify<T: Serialize + CryptoHash>(
        &self,
        message: &T,
        public_key: &FnDsa512PublicKey,
    ) -> Result<()> {
        self.verify_arbitrary_msg(&signing_message(message)?, public_key)
    }

    pub fn verify_arbitrary_msg(
        &self,
        message: &[u8],
        public_key: &FnDsa512PublicKey,
    ) -> Result<()> {
        ensure!(
            verify(&self.0[..], &public_key.0[..], &[], message),
            "invalid FN-DSA-512 signature"
        );
        Ok(())
    }
}

/// A private key that signs transactions. It keeps the expanded key (about 26 KB), which
/// signs about twice as fast.
pub struct FnDsa512PrivateKey {
    key: ExpandedSigningKey,
    public_key: FnDsa512PublicKey,
    /// Secret input for the per-message signing seeds, so signing is deterministic.
    seed_key: [u8; 32],
}

impl FnDsa512PrivateKey {
    /// Derives a key pair from a 32-byte seed. Key generation takes milliseconds.
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let key_pair = keygen_from_seed(seed);
        let key = ExpandedSigningKey::new(key_pair.signing_key.as_slice())
            .expect("a freshly generated FN-DSA signing key must decode");
        let mut seed_key_input = SIGN_SEED_DOMAIN.to_vec();
        seed_key_input.extend_from_slice(seed);
        Self {
            key,
            public_key: FnDsa512PublicKey(Arc::new(key_pair.verifying_key)),
            seed_key: *HashValue::sha3_256_of(&seed_key_input),
        }
    }

    pub fn public_key(&self) -> &FnDsa512PublicKey {
        &self.public_key
    }

    pub fn sign<T: Serialize + CryptoHash>(&self, message: &T) -> Result<FnDsa512Signature> {
        Ok(self.sign_arbitrary_message(&signing_message(message)?))
    }

    pub fn sign_arbitrary_message(&self, message: &[u8]) -> FnDsa512Signature {
        let mut seed = [0u8; SIGN_SEED_LENGTH];
        for (index, chunk) in seed.chunks_mut(HashValue::LENGTH).enumerate() {
            let mut input = self.seed_key.to_vec();
            input.push(index as u8);
            input.extend_from_slice(message);
            chunk.copy_from_slice(&HashValue::sha3_256_of(&input).as_ref()[..chunk.len()]);
        }
        FnDsa512Signature(Arc::new(self.key.sign(&[], message, &seed)))
    }
}

impl fmt::Debug for FnDsa512PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FnDsa512PrivateKey(<elided>)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_crypto::test_utils::TestAptosCrypto;

    #[test]
    fn sign_verify_and_serde() {
        let private_key = FnDsa512PrivateKey::from_seed(&[4u8; 32]);
        let message = TestAptosCrypto("txn".to_string());
        let signature = private_key.sign(&message).unwrap();
        signature
            .verify(&message, private_key.public_key())
            .unwrap();
        assert!(signature
            .verify(
                &TestAptosCrypto("other".to_string()),
                private_key.public_key()
            )
            .is_err());
        let other_key = FnDsa512PrivateKey::from_seed(&[5u8; 32]);
        assert!(signature.verify(&message, other_key.public_key()).is_err());

        let bytes = bcs::to_bytes(&signature).unwrap();
        assert_eq!(bytes.len(), FnDsa512Signature::LENGTH + 2);
        assert_eq!(
            bcs::from_bytes::<FnDsa512Signature>(&bytes).unwrap(),
            signature
        );
        let public_key = private_key.public_key().clone();
        let bytes = bcs::to_bytes(&public_key).unwrap();
        assert_eq!(
            bcs::from_bytes::<FnDsa512PublicKey>(&bytes).unwrap(),
            public_key
        );
        let json = serde_json::to_string(&public_key).unwrap();
        assert_eq!(
            serde_json::from_str::<FnDsa512PublicKey>(&json).unwrap(),
            public_key
        );
        assert!(
            bcs::from_bytes::<FnDsa512Signature>(&bcs::to_bytes(&public_key).unwrap()).is_err()
        );
    }
}
