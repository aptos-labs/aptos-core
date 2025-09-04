// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

macro_rules! dealt_secret_key_impl {
    (
        $GT_PROJ_NUM_BYTES:ident,
        $gt_proj_from_bytes:ident,
        $gt_multi_exp:ident,
        $GTProjective:ident,
        $gt:ident
    ) => {
        use crate::{
            algebra::lagrange::lagrange_coefficients,
            constants::$GT_PROJ_NUM_BYTES,
            pvss::{
                dealt_secret_key_share::$gt::DealtSecretKeyShare, player::Player,
                threshold_config::ThresholdConfig, traits, traits::SecretSharingConfig,
            },
            utils::{serialization::$gt_proj_from_bytes, $gt_multi_exp},
        };
        use velor_crypto::CryptoMaterialError;
        use velor_crypto_derive::{SilentDebug, SilentDisplay};
        use blstrs::{$GTProjective, Scalar};
        use ff::Field;
        use more_asserts::{assert_ge, assert_le};

        /// The size of a serialized *dealt secret key*.
        pub(crate) const DEALT_SK_NUM_BYTES: usize = $GT_PROJ_NUM_BYTES;

        /// The *dealt secret key* that will be output by the PVSS reconstruction algorithm. This will be of
        /// a different type than the *input secret* that was given as input to PVSS dealing.
        ///
        /// This secret key will never be reconstructed in plaintext. Instead, we will use a simple/efficient
        /// multiparty computation protocol to reconstruct a function of this secret (e.g., a verifiable
        /// random function evaluated on an input `x` under this secret).
        ///
        /// NOTE: We do not implement (de)serialization for this because a dealt secret key `sk` will never be
        /// materialized in our protocol. Instead, we always use some form of efficient multi-party computation
        /// MPC protocol to materialize a function of `sk`, such as `f(sk, m)` where `f` is a verifiable random
        /// function (VRF), for example.
        #[derive(SilentDebug, SilentDisplay, PartialEq, Clone)]
        pub struct DealtSecretKey {
            /// A group element $\hat{h}^a \in G$, where $G$ is $G_1$, $G_2$ or $G_T$
            h_hat: $GTProjective,
        }

        #[cfg(feature = "assert-private-keys-not-cloneable")]
        static_assertions::assert_not_impl_any!(DealtSecretKey: Clone);

        //
        // DealtSecretKey implementation & traits
        //

        impl DealtSecretKey {
            pub fn new(h_hat: $GTProjective) -> Self {
                Self { h_hat }
            }

            pub fn to_bytes(&self) -> [u8; DEALT_SK_NUM_BYTES] {
                self.h_hat.to_compressed()
            }

            pub fn as_group_element(&self) -> &$GTProjective {
                &self.h_hat
            }
        }

        // impl fmt::Debug for DealtSecretKey {
        //     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        //         write!(f, "{}", hex::encode(self.to_bytes()))
        //     }
        // }

        impl TryFrom<&[u8]> for DealtSecretKey {
            type Error = CryptoMaterialError;

            fn try_from(bytes: &[u8]) -> std::result::Result<DealtSecretKey, Self::Error> {
                $gt_proj_from_bytes(bytes).map(|h_hat| DealtSecretKey { h_hat })
            }
        }

        impl traits::Reconstructable<ThresholdConfig> for DealtSecretKey {
            type Share = DealtSecretKeyShare;

            /// Reconstructs the `DealtSecretKey` given a sufficiently-large subset of shares from players.
            /// Mainly used for testing the PVSS transcript dealing and decryption.
            fn reconstruct(sc: &ThresholdConfig, shares: &Vec<(Player, Self::Share)>) -> Self {
                assert_ge!(shares.len(), sc.get_threshold());
                assert_le!(shares.len(), sc.get_total_num_players());

                let ids = shares.iter().map(|(p, _)| p.id).collect::<Vec<usize>>();
                let lagr = lagrange_coefficients(
                    sc.get_batch_evaluation_domain(),
                    ids.as_slice(),
                    &Scalar::ZERO,
                );
                let bases = shares
                    .iter()
                    .map(|(_, share)| share.0.h_hat)
                    .collect::<Vec<$GTProjective>>();

                // println!();
                // println!("Lagrange IDs: {:?}", ids);
                // println!("Lagrange coeffs");
                // for l in lagr.iter() {
                // println!(" + {}", hex::encode(l.to_bytes_le()));
                // }
                // println!("Bases: ");
                // for b in bases.iter() {
                // println!(" + {}", hex::encode(b.to_bytes()));
                // }

                assert_eq!(lagr.len(), bases.len());

                DealtSecretKey {
                    h_hat: $gt_multi_exp(bases.as_slice(), lagr.as_slice()),
                }
            }
        }
    };
}

pub mod g1 {
    dealt_secret_key_impl!(
        G1_PROJ_NUM_BYTES,
        g1_proj_from_bytes,
        g1_multi_exp,
        G1Projective,
        g1
    );
}

pub mod g2 {
    // dealt_secret_key_impl!(
    //     G2_PROJ_NUM_BYTES,
    //     g2_proj_from_bytes,
    //     g2_multi_exp,
    //     G2Projective,
    //     g2
    // );
}
