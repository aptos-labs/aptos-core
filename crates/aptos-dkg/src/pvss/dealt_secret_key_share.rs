// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

macro_rules! dealt_secret_key_share_impl {
    ($GTProjective:ident, $gt:ident) => {
        use crate::pvss::dealt_secret_key::$gt::{DealtSecretKey, DEALT_SK_NUM_BYTES};
        use aptos_crypto::{
            CryptoMaterialError, ValidCryptoMaterial, ValidCryptoMaterialStringExt,
        };
        use aptos_crypto_derive::{DeserializeKey, SerializeKey, SilentDebug, SilentDisplay};
        use blstrs::$GTProjective;

        /// The size of a serialized *dealt secret key share*.
        const DEALT_SK_SHARE_NUM_BYTES: usize = DEALT_SK_NUM_BYTES;

        /// A player's *share* of the secret key that was dealt via the PVSS transcript.
        #[derive(DeserializeKey, SerializeKey, SilentDisplay, SilentDebug, PartialEq, Clone)]
        pub struct DealtSecretKeyShare(pub(crate) DealtSecretKey);

        #[cfg(feature = "assert-private-keys-not-cloneable")]
        static_assertions::assert_not_impl_any!(DealtSecretKeyShare: Clone);

        //
        // DealtSecretKeyShare implementation & traits
        //

        impl DealtSecretKeyShare {
            pub fn new(dealt_sk: DealtSecretKey) -> Self {
                DealtSecretKeyShare(dealt_sk)
            }

            pub fn to_bytes(&self) -> [u8; DEALT_SK_SHARE_NUM_BYTES] {
                self.0.to_bytes()
            }

            pub fn as_group_element(&self) -> &$GTProjective {
                self.0.as_group_element()
            }
        }

        impl ValidCryptoMaterial for DealtSecretKeyShare {
            const AIP_80_PREFIX: &'static str = "";
            fn to_bytes(&self) -> Vec<u8> {
                self.to_bytes().to_vec()
            }
        }

        // impl fmt::Debug for DealtSecretKeyShare {
        //     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        //         write!(f, "{}", hex::encode(self.to_bytes()))
        //     }
        // }

        impl TryFrom<&[u8]> for DealtSecretKeyShare {
            type Error = CryptoMaterialError;

            fn try_from(bytes: &[u8]) -> std::result::Result<DealtSecretKeyShare, Self::Error> {
                DealtSecretKey::try_from(bytes).map(|sk| DealtSecretKeyShare(sk))
            }
        }
    };
}

pub mod g1 {
    dealt_secret_key_share_impl!(G1Projective, g1);
}

pub mod g2 {
    //dealt_secret_key_share_impl!(G2Projective, g2);
}
