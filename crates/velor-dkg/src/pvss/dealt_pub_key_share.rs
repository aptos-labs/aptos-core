// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// NOTE: I don't think we need this DealtPubKey[Share] anymore, since we never implement any traits
// on it, unlike the DealtSecretKey[Share]. We will keep it in case we want to implement the
// `Reconstructable` trait later on though.

macro_rules! dealt_pub_key_share_impl {
    ($GTProjective:ident, $gt:ident) => {
        use crate::pvss::dealt_pub_key::$gt::{DealtPubKey, DEALT_PK_NUM_BYTES};
        use velor_crypto::{
            CryptoMaterialError, ValidCryptoMaterial, ValidCryptoMaterialStringExt,
        };
        use velor_crypto_derive::{DeserializeKey, SerializeKey};
        use blstrs::$GTProjective;

        /// The size of a serialized *dealt public key share*.
        pub(crate) const DEALT_PK_SHARE_NUM_BYTES: usize = DEALT_PK_NUM_BYTES;

        /// A player's *share* of the *dealt public key* from above.
        #[derive(DeserializeKey, Clone, Debug, SerializeKey, PartialEq, Eq)]
        pub struct DealtPubKeyShare(pub(crate) DealtPubKey);

        //
        // DealtPublicKeyShare
        //

        impl DealtPubKeyShare {
            pub fn new(dealt_pk: DealtPubKey) -> Self {
                DealtPubKeyShare(dealt_pk)
            }

            pub fn to_bytes(&self) -> [u8; DEALT_PK_SHARE_NUM_BYTES] {
                self.0.to_bytes()
            }

            pub fn as_group_element(&self) -> &$GTProjective {
                self.0.as_group_element()
            }
        }

        impl ValidCryptoMaterial for DealtPubKeyShare {
            const AIP_80_PREFIX: &'static str = "";

            fn to_bytes(&self) -> Vec<u8> {
                self.to_bytes().to_vec()
            }
        }

        impl TryFrom<&[u8]> for DealtPubKeyShare {
            type Error = CryptoMaterialError;

            /// Deserialize a `DealtPublicKeyShare`.
            fn try_from(bytes: &[u8]) -> std::result::Result<DealtPubKeyShare, Self::Error> {
                DealtPubKey::try_from(bytes).map(|pk| DealtPubKeyShare(pk))
            }
        }
    };
}

pub mod g1 {
    //dealt_pub_key_share_impl!(G1Projective, g1);
}

pub mod g2 {
    dealt_pub_key_share_impl!(G2Projective, g2);
}
