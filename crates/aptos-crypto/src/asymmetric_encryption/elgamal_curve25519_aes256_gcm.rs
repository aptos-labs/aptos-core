// Copyright © Aptos Foundation

use crate::{
    asymmetric_encryption::AsymmetricEncryption,
    elgamal,
    elgamal::{curve25519::Curve25519, ElGamalFriendlyGroup},
};
use aes_gcm::{
    aead::{
        rand_core::{CryptoRng as AeadCryptoRng, RngCore as AeadRngCore},
        Aead, Nonce,
    },
    AeadCore, Aes256Gcm, Key, KeyInit,
};
use anyhow::{anyhow, bail, ensure};
use curve25519_dalek::{edwards::CompressedEdwardsY, scalar::Scalar};
use rand_core::{CryptoRng, RngCore};
use sha3::{Digest, Sha3_256};

/// An asymmetric encryption which:
/// - uses AES-256-GCM to encrypt the original variable-length input, where the symmetric key is freshly sampled;
/// - uses ElGamal over the group that supports ED25519 signatures to encrypt the symmetric key.
pub struct ElGamalCurve25519Aes256Gcm {}

impl ElGamalCurve25519Aes256Gcm {
    fn hash_group_element_to_aes_key(element: &CompressedEdwardsY) -> Vec<u8> {
        let mut hasher = Sha3_256::new();
        hasher.update(b"DST__AES_KEY_DERIVATION");
        hasher.update(element.to_bytes());
        hasher.finalize().to_vec()
    }
}

const SCHEME_NAME: &str = "ElGamalCurve25519Aes256Gcm";

impl AsymmetricEncryption for ElGamalCurve25519Aes256Gcm {
    fn scheme_name() -> String {
        SCHEME_NAME.to_string()
    }

    fn key_gen<R: CryptoRng + RngCore>(rng: &mut R) -> (Vec<u8>, Vec<u8>) {
        let (sk, pk) = elgamal::key_gen::<Curve25519, _>(rng);
        let sk_bytes = sk.to_bytes().to_vec();
        let pk_bytes = pk.compress().to_bytes().to_vec();
        (sk_bytes, pk_bytes)
    }

    fn enc<R1: CryptoRng + RngCore, R2: AeadCryptoRng + AeadRngCore>(
        main_rng: &mut R1,
        aead_rng: &mut R2,
        pk: &[u8],
        msg: &[u8],
    ) -> anyhow::Result<Vec<u8>> {
        if pk.len() != 32 {
            bail!("ElGamalCurve25519Aes256Gcm enc failed with incorrect pk length");
        }
        let pk = CompressedEdwardsY::from_slice(pk)
            .decompress()
            .ok_or_else(|| {
                anyhow!("ElGamalCurve25519Aes256Gcm enc failed with invalid pk element")
            })?;

        ensure!(
            pk.is_torsion_free(),
            "ElGamalCurve25519Aes256Gcm enc failed with non-prime-order PK"
        );

        let aes_key_g1 = Curve25519::rand_element(main_rng);
        let (elgamal_ciphertext_0, elgamal_ciphertext_1) =
            elgamal::encrypt::<Curve25519, _>(main_rng, &pk, &aes_key_g1);
        let aes_key_bytes = Self::hash_group_element_to_aes_key(&aes_key_g1.compress());
        let key = Key::<Aes256Gcm>::from_slice(aes_key_bytes.as_slice());
        let cipher = Aes256Gcm::new(key);
        let nonce = Aes256Gcm::generate_nonce(aead_rng);
        let nonce_bytes = nonce.to_vec();
        ensure!(
            12 == nonce_bytes.len(),
            "ElGamalCurve25519Aes256Gcm enc failed with unexpected nonce len"
        );

        let aes_ciphertext = cipher.encrypt(&nonce, msg.as_ref()).map_err(|e| {
            anyhow!(
                "ElGamalCurve25519Aes256Gcm enc failed with aes error: {}",
                e
            )
        })?;

        let elgamal_ciphertext_0_bytes = elgamal_ciphertext_0.compress().to_bytes().to_vec();
        let elgamal_ciphertext_1_bytes = elgamal_ciphertext_1.compress().to_bytes().to_vec();

        let serialized = [
            elgamal_ciphertext_0_bytes, // 32 bytes
            elgamal_ciphertext_1_bytes, // 32 bytes
            nonce_bytes,                // 12 bytes
            aes_ciphertext,             // variable length
        ]
            .concat();

        Ok(serialized)
    }

    fn dec(sk: &[u8], ciphertext: &[u8]) -> anyhow::Result<Vec<u8>> {
        let sk = <[u8; 32]>::try_from(sk.to_vec()).map_err(|_e| {
            anyhow!("ElGamalCurve25519Aes256Gcm dec failed with incorrect sk length")
        })?;
        let sk_scalar = Scalar::from_canonical_bytes(sk).ok_or_else(|| {
            anyhow!("ElGamalCurve25519Aes256Gcm dec failed with sk deserialization error")
        })?;
        ensure!(
            ciphertext.len() >= 76,
            "ElGamalCurve25519Aes256Gcm dec failed with invalid ciphertext length"
        );
        let c0 = CompressedEdwardsY::from_slice(&ciphertext[0..32])
            .decompress()
            .ok_or_else(|| {
                anyhow!("ElGamalCurve25519Aes256Gcm dec failed with invalid c0 element")
            })?;

        ensure!(
            c0.is_torsion_free(),
            "ElGamalCurve25519Aes256Gcm dec failed with non-prime-order c0"
        );

        let c1 = CompressedEdwardsY::from_slice(&ciphertext[32..64])
            .decompress()
            .ok_or_else(|| {
                anyhow!("ElGamalCurve25519Aes256Gcm dec failed with invalid c1 element")
            })?;

        ensure!(
            c1.is_torsion_free(),
            "ElGamalCurve25519Aes256Gcm dec failed with non-prime-order c1"
        );

        let aes_key_element = elgamal::decrypt::<Curve25519>(&sk_scalar, &c0, &c1).compress();
        let aes_key_bytes = Self::hash_group_element_to_aes_key(&aes_key_element);
        let key = Key::<Aes256Gcm>::from_slice(aes_key_bytes.as_slice());
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::<Aes256Gcm>::from_slice(&ciphertext[64..76]);
        let plaintext = cipher.decrypt(nonce, &ciphertext[76..]).map_err(|e| {
            anyhow!("ElGamalCurve25519Aes256Gcm dec failed with aes decryption error: {e}")
        })?;
        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use crate::asymmetric_encryption::{
        elgamal_curve25519_aes256_gcm::ElGamalCurve25519Aes256Gcm, AsymmetricEncryption,
    };

    #[test]
    fn gen_enc_dec() {
        let mut main_rng = rand_core::OsRng;
        let mut aead_rng = aes_gcm::aead::OsRng;
        let (sk, pk) = ElGamalCurve25519Aes256Gcm::key_gen(&mut main_rng);
        let msg = b"hello world again and again and again and again and again and again and again"
            .to_vec();
        let ciphertext = ElGamalCurve25519Aes256Gcm::enc(
            &mut main_rng,
            &mut aead_rng,
            pk.as_slice(),
            msg.as_slice(),
        )
            .unwrap();
        assert_eq!(
            msg,
            ElGamalCurve25519Aes256Gcm::dec(sk.as_slice(), ciphertext.as_slice()).unwrap()
        );
    }
}
