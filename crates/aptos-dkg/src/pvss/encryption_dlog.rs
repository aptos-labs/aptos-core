// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Implements public parameters $h \in G$ for a simple DLOG-based encryption scheme.
macro_rules! encryption_dlog_pp_impl {
    ($GT_PROJ_NUM_BYTES:ident, $gt_proj_from_bytes:ident, $GTProjective:ident) => {
        pub const PUBLIC_PARAMS_NUM_BYTES: usize = $GT_PROJ_NUM_BYTES;

        /// The public parameters used in the encryption scheme.
        #[derive(DeserializeKey, Clone, SerializeKey, PartialEq, Debug, Eq)]
        pub struct PublicParameters {
            /// A group element $h \in G$, where $G$ is $G_1$, $G_2$ or $G_T$.
            h: $GTProjective,
        }

        impl PublicParameters {
            pub fn new(h: $GTProjective) -> Self {
                Self { h }
            }

            pub fn to_bytes(&self) -> [u8; $GT_PROJ_NUM_BYTES] {
                self.h.to_compressed()
            }

            pub fn as_group_element(&self) -> &$GTProjective {
                &self.h
            }
        }

        impl TryFrom<&[u8]> for PublicParameters {
            type Error = CryptoMaterialError;

            fn try_from(bytes: &[u8]) -> std::result::Result<PublicParameters, Self::Error> {
                $gt_proj_from_bytes(bytes).map(|h| PublicParameters { h })
            }
        }

        impl ValidCryptoMaterial for PublicParameters {
            const AIP_80_PREFIX: &'static str = "";

            fn to_bytes(&self) -> Vec<u8> {
                self.to_bytes().to_vec()
            }
        }
    };
}

/// Implements structs for SKs and PKs, where SKs are scalars and the PKs can be implemented to be
/// any function of the SK via the `crate::pvss::traits::Convert` trait (e.g., $pk = h^{sk^{-1}}$).
macro_rules! encryption_dlog_keys_impl {
    ($GT_PROJ_NUM_BYTES:ident, $gt_proj_from_bytes:ident, $GTProjective:ident) => {
        use crate::{
            constants::{$GT_PROJ_NUM_BYTES, SCALAR_NUM_BYTES},
            utils::{
                random::random_scalar,
                serialization::{scalar_from_bytes_le, $gt_proj_from_bytes},
            },
        };
        use aptos_crypto::{
            CryptoMaterialError, Length, Uniform, ValidCryptoMaterial, ValidCryptoMaterialStringExt,
        };
        use aptos_crypto_derive::{DeserializeKey, SerializeKey, SilentDebug, SilentDisplay};
        use blstrs::{$GTProjective, Scalar};
        use std::{
            fmt,
            hash::{Hash, Hasher},
        };

        //
        // Constants
        //

        /// The length in bytes of a decryption key.
        pub const DECRYPT_KEY_NUM_BYTES: usize = SCALAR_NUM_BYTES;

        /// The length in bytes of an encryption key.
        pub const ENCRYPT_KEY_NUM_BYTES: usize = $GT_PROJ_NUM_BYTES;

        //
        // Structs
        //

        /// The *encryption (public)* key used to encrypt shares of the dealt secret for each PVSS player.
        #[derive(DeserializeKey, SerializeKey, Clone, Eq)]
        pub struct EncryptPubKey {
            /// A group element $h^{dk^{-1}} \in G_1$.
            pub(crate) ek: $GTProjective,
        }

        /// The *decryption (secret) key* used by each PVSS player do decrypt their share of the dealt secret.
        #[derive(DeserializeKey, SerializeKey, SilentDisplay, SilentDebug)]
        pub struct DecryptPrivKey {
            /// A scalar $dk \in F$.
            pub(crate) dk: Scalar,
        }

        #[cfg(feature = "assert-private-keys-not-cloneable")]
        static_assertions::assert_not_impl_any!(DecryptPrivKey: Clone);

        //
        // DecryptPrivKey
        //

        impl DecryptPrivKey {
            pub fn to_bytes(&self) -> [u8; DECRYPT_KEY_NUM_BYTES] {
                self.dk.to_bytes_le()
            }

            pub fn to_bytes_be(&self) -> [u8; DECRYPT_KEY_NUM_BYTES] {
                self.dk.to_bytes_be()
            }

            pub fn from_bytes_be(bytes: &[u8; DECRYPT_KEY_NUM_BYTES]) -> Self {
                Self {
                    dk: Scalar::from_bytes_be(bytes).unwrap(),
                }
            }
        }

        // impl fmt::Debug for DecryptPrivKey {
        //     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        //         write!(f, "{}", hex::encode(self.to_bytes()))
        //     }
        // }

        impl Length for DecryptPrivKey {
            fn length(&self) -> usize {
                DECRYPT_KEY_NUM_BYTES
            }
        }

        // impl PrivateKey for DecryptPrivKey {
        //     type PublicKeyMaterial = EncryptPubKey;
        // }

        impl ValidCryptoMaterial for DecryptPrivKey {
            const AIP_80_PREFIX: &'static str = "";
            fn to_bytes(&self) -> Vec<u8> {
                self.to_bytes().to_vec()
            }
        }

        impl TryFrom<&[u8]> for DecryptPrivKey {
            type Error = CryptoMaterialError;

            fn try_from(bytes: &[u8]) -> std::result::Result<DecryptPrivKey, Self::Error> {
                scalar_from_bytes_le(bytes).map(|dk| DecryptPrivKey { dk })
            }
        }

        impl Uniform for DecryptPrivKey {
            fn generate<R>(rng: &mut R) -> Self
            where
                R: rand_core::RngCore + rand::Rng + rand_core::CryptoRng + rand::CryptoRng,
            {
                DecryptPrivKey {
                    dk: random_scalar(rng),
                }
            }
        }

        //
        // EncryptPubKey
        //

        impl EncryptPubKey {
            /// Serializes an encryption key.
            pub fn to_bytes(&self) -> [u8; ENCRYPT_KEY_NUM_BYTES] {
                self.ek.to_compressed()
            }
        }

        impl fmt::Debug for EncryptPubKey {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", hex::encode(self.to_bytes()))
            }
        }

        impl From<&EncryptPubKey> for $GTProjective {
            fn from(ek: &EncryptPubKey) -> Self {
                ek.ek
            }
        }

        impl PartialEq for EncryptPubKey {
            fn eq(&self, other: &Self) -> bool {
                self.to_bytes() == other.to_bytes()
            }
        }

        impl Hash for EncryptPubKey {
            fn hash<H: Hasher>(&self, state: &mut H) {
                state.write(self.to_bytes().as_slice())
            }
        }

        impl ValidCryptoMaterial for EncryptPubKey {
            const AIP_80_PREFIX: &'static str = "";
            fn to_bytes(&self) -> Vec<u8> {
                self.to_bytes().to_vec()
            }
        }

        impl TryFrom<&[u8]> for EncryptPubKey {
            type Error = CryptoMaterialError;

            /// Deserialize an `EncryptPubKey`. This method will check that the public key is in the
            /// (prime-order) group.
            fn try_from(bytes: &[u8]) -> std::result::Result<EncryptPubKey, Self::Error> {
                $gt_proj_from_bytes(bytes).map(|ek| EncryptPubKey { ek })
            }
        }
    };
}

pub mod g1 {
    // PPs not needed, for now.
    encryption_dlog_keys_impl!(G1_PROJ_NUM_BYTES, g1_proj_from_bytes, G1Projective);
}

pub mod g2 {
    encryption_dlog_pp_impl!(G2_PROJ_NUM_BYTES, g2_proj_from_bytes, G2Projective);
    encryption_dlog_keys_impl!(G2_PROJ_NUM_BYTES, g2_proj_from_bytes, G2Projective);
}
