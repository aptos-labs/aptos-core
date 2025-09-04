// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::pvss::{
    das, dealt_pub_key::g2::DealtPubKey, dealt_secret_key::g1::DealtSecretKey,
    input_secret::InputSecret, traits, traits::HasEncryptionPublicParams,
};
use std::ops::Mul;

//
// InputSecret implementation
//

impl traits::Convert<DealtSecretKey, das::PublicParameters> for InputSecret {
    fn to(&self, pp: &das::PublicParameters) -> DealtSecretKey {
        DealtSecretKey::new(
            pp.get_encryption_public_params()
                .message_base()
                .mul(self.get_secret_a()),
        )
    }
}

impl traits::Convert<DealtPubKey, das::PublicParameters> for InputSecret {
    /// Computes the public key associated with the given input secret.
    /// NOTE: In the SCRAPE PVSS, a `DealtPublicKey` cannot be computed from a `DealtSecretKey` directly.
    fn to(&self, pp: &das::PublicParameters) -> DealtPubKey {
        DealtPubKey::new(pp.get_commitment_base().mul(self.get_secret_a()))
    }
}

#[cfg(test)]
mod test {}
