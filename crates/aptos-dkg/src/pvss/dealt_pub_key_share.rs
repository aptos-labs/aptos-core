// Copyright © Aptos Foundation

macro_rules! dealt_pub_key_share_impl {
    (
        $gt:ident
    ) => {
        use crate::pvss::dealt_pub_key::$gt::DealtPubKey;
        use crate::pvss::dealt_pub_key::$gt::DEALT_PK_NUM_BYTES;

        use aptos_crypto::{
            CryptoMaterialError, ValidCryptoMaterial, ValidCryptoMaterialStringExt,
        };
        use aptos_crypto_derive::{DeserializeKey, SerializeKey};

        /// The size of a serialized *dealt public key share*.
        pub(crate) const DEALT_PK_SHARE_NUM_BYTES: usize = DEALT_PK_NUM_BYTES;

        /// A player's *share* of the *dealt public key* from above.
        #[derive(DeserializeKey, Clone, SerializeKey)]
        pub struct DealtPubKeyShare(pub(crate) DealtPubKey);

        //
        // DealtPublicKeyShare
        //

        impl DealtPubKeyShare {
            pub fn to_bytes(&self) -> [u8; DEALT_PK_SHARE_NUM_BYTES] {
                self.0.to_bytes()
            }
        }

        impl ValidCryptoMaterial for DealtPubKeyShare {
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
    dealt_pub_key_share_impl!(g1);
}

pub mod g2 {
    dealt_pub_key_share_impl!(g2);
}
