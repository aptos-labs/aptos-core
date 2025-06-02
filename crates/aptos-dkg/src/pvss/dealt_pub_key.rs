// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

macro_rules! dealt_pub_key_impl {
    ($GT_PROJ_NUM_BYTES:ident, $gt_proj_from_bytes:ident, $GTProjective:ident) => {
        use crate::{constants::$GT_PROJ_NUM_BYTES, utils::serialization::$gt_proj_from_bytes};
        use aptos_crypto::{
            CryptoMaterialError, ValidCryptoMaterial, ValidCryptoMaterialStringExt,
        };
        use aptos_crypto_derive::{DeserializeKey, SerializeKey};
        use blstrs::$GTProjective;

        /// The size of a serialized *dealt public key*.
        pub(crate) const DEALT_PK_NUM_BYTES: usize = $GT_PROJ_NUM_BYTES;

        /// The *dealt public key* associated with the secret key that was dealt via the PVSS transcript.
        #[derive(DeserializeKey, Clone, Debug, SerializeKey, PartialEq, Eq)]
        pub struct DealtPubKey {
            /// A group element $g_1^a \in G$, where $G$ is $G_1$, $G_2$ or $G_T$
            g_a: $GTProjective,
        }

        //
        // DealtPublicKey
        //

        impl DealtPubKey {
            pub fn new(g_a: $GTProjective) -> Self {
                Self { g_a }
            }

            pub fn to_bytes(&self) -> [u8; DEALT_PK_NUM_BYTES] {
                self.g_a.to_compressed()
            }

            pub fn as_group_element(&self) -> &$GTProjective {
                &self.g_a
            }
        }

        impl ValidCryptoMaterial for DealtPubKey {
            const AIP_80_PREFIX: &'static str = "";

            fn to_bytes(&self) -> Vec<u8> {
                self.to_bytes().to_vec()
            }
        }

        impl TryFrom<&[u8]> for DealtPubKey {
            type Error = CryptoMaterialError;

            fn try_from(bytes: &[u8]) -> std::result::Result<DealtPubKey, Self::Error> {
                $gt_proj_from_bytes(bytes).map(|g_a| DealtPubKey { g_a })
            }
        }
    };
}

pub mod g1 {
    // dealt_pub_key_impl!(G1_PROJ_NUM_BYTES, g1_proj_from_bytes, G1Projective);
}

pub mod g2 {
    dealt_pub_key_impl!(G2_PROJ_NUM_BYTES, g2_proj_from_bytes, G2Projective);
}
