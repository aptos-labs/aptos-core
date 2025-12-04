// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::pvss::{
    encryption_dlog::g1::{DecryptPrivKey, EncryptPubKey},
    encryption_elgamal::g1::PublicParameters,
    traits,
};
use std::ops::Mul;

impl traits::Convert<EncryptPubKey, PublicParameters> for DecryptPrivKey {
    /// Given a decryption key $dk$, computes its associated encryption key $g_1^{dk}$
    fn to(&self, pp: &PublicParameters) -> EncryptPubKey {
        EncryptPubKey {
            ek: pp.pubkey_base().mul(self.dk),
        }
    }
}
