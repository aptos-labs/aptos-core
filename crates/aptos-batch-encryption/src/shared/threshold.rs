// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::group::{Fr, G1Affine};
use rand_core::RngCore;
use ark_std::UniformRand;


pub struct MasterSecretKeyShare {
    sign_sk: ShamirShare,
    pub digest_setup: DigestKey,
    pub batch_size: usize,
}

pub struct ThresholdPublicKey {
    pub pk: PublicKey,
    shamir_params: ShamirParams,
}

pub fn keygen(rng: &mut impl RngCore, n: usize, t: usize, batch_size: usize) -> (ThresholdPublicKey, Vec<MasterSecretKeyShare>) {

    let digest_setup = DigestKey::new(rng, batch_size).unwrap();


    let shamir_params = ShamirParams::new(n, t);
    let sign_sk = Fr::rand(rng);
    let sign_sk_shares = shamir_params.share(sign_sk, rng);

    let sk_shares = sign_sk_shares.iter().map(
        |share|
        MasterSecretKeyShare {
            sign_sk: *share,
            digest_setup: digest_setup.clone(),
            batch_size,
        }
        ).collect();

    let pk = ThresholdPublicKey {
        pk: MasterSecretKey { sign_sk, digest_setup: digest_setup.clone(), batch_size }.derive_public_key(),
        shamir_params
    };

    (pk, sk_shares)
}


pub struct DecryptionKeyShare(ShamirGroupShare);


impl MasterSecretKeyShare {
    pub fn derive_decryption_key_share(&self, digest: &SuccinctDigest, batch_num: usize) -> DecryptionKeyShare
    {
        let hashed_batch_num : G1Affine = hash_batch_num(batch_num);

        DecryptionKeyShare(
            ShamirGroupShare {
                x: self.sign_sk.x,
                g_y: G1Affine::from((hashed_batch_num + digest.as_g1()) * self.sign_sk.y),
            }
            )
            // can make ShamirShare::mul_with_group_elt to make this cleaner
    }
}

impl ThresholdPublicKey {
    pub fn reconstruct_decryption_key(&self, shares: &[DecryptionKeyShare]) -> DecryptionKey {
        DecryptionKey(
            self.shamir_params.reconstruct_in_exponent(
                &shares.iter().map(|DecryptionKeyShare(x)| *x).collect::<Vec<ShamirGroupShare>>()
            )
            )
    }
}



#[cfg(test)]
mod tests {
    use ark_ec::{pairing::PairingOutput, PrimeGroup};
    use ark_std::rand::thread_rng;


    use crate::{shared::digest::{Digest}, shared::ids::{RootsOfUnityIdSet, RootsOfUnityId}, variants::vanilla::{Ciphertext, Plaintext}};

    use super::*;

    #[test]
    fn threshold_decrypt_all() {
        let batch_size = 8;
        let number_of_parties = 16;
        let threshold = 8;
        let mut rng = thread_rng();

        let (tpk, msk_shares) = keygen(&mut rng, number_of_parties, threshold, batch_size);
        let pk = &tpk.pk;

        let mut ids = RootsOfUnityIdSet::with_capacity(batch_size).unwrap();

        for x in 0..batch_size {
            ids.set(x, Fr::rand(&mut rng));
        }
        let d = Digest::compute(&pk.digest_setup, ids);
        let sd = SuccinctDigest::from(&d);
        let sk_shares : Vec<DecryptionKeyShare> = msk_shares.iter().map(
            |share| share.derive_decryption_key_share(&sd, 2)).collect();
        let sk = tpk.reconstruct_decryption_key(&sk_shares);

        let cts : Vec<Ciphertext<RootsOfUnityId>> = d.ids.ids().map(
            |id| pk.encrypt(&mut rng, Plaintext(PairingOutput::generator()), id, 2)
            ).collect();

        let pts = sk.decrypt_all(
            &cts,
            &d);

        for pt in pts.iter().take(batch_size) {
            assert_eq!(*pt,Plaintext(PairingOutput::generator()));
        }

    }

}
