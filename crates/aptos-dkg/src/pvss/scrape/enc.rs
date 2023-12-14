// Copyright © Aptos Foundation

use crate::pvss::{
    encryption_dlog::g2::{DecryptPrivKey, EncryptPubKey, PublicParameters},
    traits,
};
use ff::Field;
use std::ops::Mul;

impl traits::Convert<EncryptPubKey, PublicParameters> for DecryptPrivKey {
    /// Given a decryption key $dk$, computes its associated encryption key $h^{dk^{-1}}$
    fn to(&self, pp: &PublicParameters) -> EncryptPubKey {
        EncryptPubKey {
            ek: pp.as_group_element().mul(self.dk.invert().unwrap()),
        }
    }
}
