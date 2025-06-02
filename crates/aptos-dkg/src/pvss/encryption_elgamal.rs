// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Implements public parameters $(h, g) \in G$ for an ElGamal encryption scheme where $h$
/// is the message base and $g$ is the PK base.
macro_rules! encryption_elgamal_pp_impl {
    ($GT_PROJ_NUM_BYTES:ident, $gt_proj_from_bytes:ident, $GTProjective:ident) => {
        use crate::{constants::$GT_PROJ_NUM_BYTES, utils::serialization::$gt_proj_from_bytes};
        use aptos_crypto::{
            CryptoMaterialError, ValidCryptoMaterial, ValidCryptoMaterialStringExt,
        };
        use aptos_crypto_derive::{DeserializeKey, SerializeKey};
        use blstrs::$GTProjective;

        //
        // Constants
        //

        /// The length in bytes of the public params struct.
        pub const PUBLIC_PARAMS_NUM_BYTES: usize = $GT_PROJ_NUM_BYTES * 2;

        //
        // Structs
        //

        /// The public parameters used in the encryption scheme.
        #[derive(DeserializeKey, PartialEq, Clone, SerializeKey, Eq, Debug)]
        pub struct PublicParameters {
            /// A group element $g \in G$, where $G$ is $G_1$, $G_2$ or $G_T$ used to exponentiate
            /// both the (1) ciphertext randomness and the (2) the DSK when computing its EK.
            g: $GTProjective,
            /// A group element $h \in G$ that is raised to the encrypted message
            h: $GTProjective,
        }

        impl PublicParameters {
            pub fn new(g: $GTProjective, h: $GTProjective) -> Self {
                Self { g, h }
            }

            pub fn to_bytes(&self) -> [u8; 2 * $GT_PROJ_NUM_BYTES] {
                let mut bytes = [0u8; 2 * $GT_PROJ_NUM_BYTES];

                // Copy bytes from g.to_compressed() into the first half of the bytes array.
                bytes[..$GT_PROJ_NUM_BYTES].copy_from_slice(&self.g.to_compressed());

                // Copy bytes from h.to_compressed() into the second half of the bytes array.
                bytes[$GT_PROJ_NUM_BYTES..].copy_from_slice(&self.h.to_compressed());

                bytes
            }

            pub fn pubkey_base(&self) -> &$GTProjective {
                &self.g
            }

            pub fn message_base(&self) -> &$GTProjective {
                &self.h
            }
        }

        impl ValidCryptoMaterial for PublicParameters {
            const AIP_80_PREFIX: &'static str = "";

            fn to_bytes(&self) -> Vec<u8> {
                self.to_bytes().to_vec()
            }
        }

        impl TryFrom<&[u8]> for PublicParameters {
            type Error = CryptoMaterialError;

            fn try_from(bytes: &[u8]) -> std::result::Result<PublicParameters, Self::Error> {
                let g = $gt_proj_from_bytes(&bytes[0..$GT_PROJ_NUM_BYTES])?;
                let h = $gt_proj_from_bytes(&bytes[$GT_PROJ_NUM_BYTES..])?;

                Ok(PublicParameters { g, h })
            }
        }
    };
}

pub mod g1 {
    encryption_elgamal_pp_impl!(G1_PROJ_NUM_BYTES, g1_proj_from_bytes, G1Projective);
}

pub mod g2 {
    // Not needed, for now
}
