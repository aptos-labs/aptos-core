// Copyright © Aptos Foundation

use crate::pvss::{
    dealt_pub_key::g1::DealtPubKey, dealt_secret_key::g2::DealtSecretKey,
    input_secret::InputSecret, scrape::public_parameters::PublicParameters, traits,
};
use std::ops::Mul;

//
// InputSecret implementation
//

impl traits::Convert<DealtSecretKey, PublicParameters> for InputSecret {
    fn to(&self, pp: &PublicParameters) -> DealtSecretKey {
        DealtSecretKey::new(pp.get_encryption_key_base().mul(self.get_secret_a()))
    }
}

impl traits::Convert<DealtPubKey, PublicParameters> for InputSecret {
    /// Computes the public key associated with the given input secret.
    /// NOTE: In the SCRAPE PVSS, a `DealtPublicKey` cannot be computed from a `DealtSecretKey` directly.
    fn to(&self, pp: &PublicParameters) -> DealtPubKey {
        DealtPubKey::new(pp.get_commitment_base().mul(self.get_secret_a()))
    }
}

#[cfg(test)]
mod test {}
