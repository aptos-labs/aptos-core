// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::{
    errors::BatchEncryptionError,
    group::{Fq, G1Affine, G2Affine},
    traits::Plaintext,
};
use aes_gcm::{aead::Aead as _, aes::Aes128, AeadCore, Aes128Gcm, AesGcm, Key, KeySizeUser, Nonce};
use anyhow::Result;
use ark_ec::{short_weierstrass::SWCurveConfig, AffineRepr};
use ark_ff::{
    field_hashers::{DefaultFieldHasher, HashToField},
    Field,
};
use ark_serialize::CanonicalSerialize as _;
use ark_std::{
    rand::{CryptoRng, RngCore},
    Zero,
};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::{
    digest::{
        consts::{B0, B1, U16},
        generic_array::{functional::FunctionalSequence as _, sequence::Split, GenericArray},
        typenum::{UInt, UTerm},
        OutputSizeUser,
    },
    Sha256,
};

type KeySize = <Aes128 as KeySizeUser>::KeySize;
type SymmetricCipher = Aes128Gcm;
type SymmetricNonce =
    Nonce<<AesGcm<Aes128, UInt<UInt<UInt<UInt<UTerm, B1>, B1>, B0>, B0>> as AeadCore>::NonceSize>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct SymmetricKey(GenericArray<u8, KeySize>);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct OneTimePad(GenericArray<u8, KeySize>);
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub struct OneTimePaddedKey(GenericArray<u8, KeySize>);

impl OneTimePaddedKey {
    #[cfg(test)]
    pub(crate) fn blank_for_testing() -> Self {
        let blank = vec![0; 16];
        Self(GenericArray::clone_from_slice(blank.as_slice()))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct SymmetricCiphertext {
    nonce: SymmetricNonce,
    #[serde(with = "serde_bytes")]
    ct_body: Vec<u8>,
}

impl SymmetricCiphertext {
    #[cfg(test)]
    pub(crate) fn blank_for_testing() -> Self {
        Self {
            nonce: SymmetricNonce::default(),
            ct_body: vec![],
        }
    }
}

impl OneTimePad {
    /// Take some source bytes that are high-entropy (but not necessarily uniformly-distributed),
    /// and generate a one-time pad of [`KeySize`] length that is indistingushable from uniform
    /// random.
    pub fn from_source_bytes(otp_source: impl AsRef<[u8]>) -> Self {
        let otp = hmac_kdf(otp_source);
        let (otp_first_half, _): (GenericArray<u8, U16>, GenericArray<u8, U16>) = otp.split();
        Self(otp_first_half)
    }

    pub fn pad_key(&self, value: &SymmetricKey) -> OneTimePaddedKey {
        OneTimePaddedKey(self.0.zip(value.0, |p, k| p ^ k))
    }

    pub fn unpad_key(&self, value: &OneTimePaddedKey) -> SymmetricKey {
        SymmetricKey(self.0.zip(value.0, |p, k| p ^ k))
    }
}

impl SymmetricKey {
    // Generate a random symmetric key.
    pub fn new<R: CryptoRng + RngCore>(rng: &mut R) -> Self {
        use aes_gcm::KeyInit as _; // putting this in the global scope causes Hmac<Sha256> to be
                                   // ambiguous for some reason

        // unwrap is safe here b/c the above array is of the correct size
        Self(Aes128Gcm::generate_key(rng))
    }

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes.into())
    }

    /// Encrypt any plaintext that is serializable into bytes.
    pub fn encrypt<R: RngCore + CryptoRng>(
        &self,
        rng: &mut R,
        plaintext: &impl Plaintext,
    ) -> Result<SymmetricCiphertext> {
        use aes_gcm::KeyInit as _; // putting this in the global scope causes Hmac<Sha256> to be
                                   // ambiguous for some reason

        let key: &Key<SymmetricCipher> = &self.0;

        let cipher = SymmetricCipher::new(key);
        let nonce = SymmetricCipher::generate_nonce(rng); // 96-bits; unique per message
                                                          //
        let mut plaintext_bytes = Vec::new();
        bcs::serialize_into(&mut plaintext_bytes, &plaintext)
            .map_err(|_| BatchEncryptionError::SerializationError)?;

        Ok(SymmetricCiphertext {
            nonce,
            ct_body: cipher
                .encrypt(&nonce, plaintext_bytes.as_ref())
                .map_err(|_| BatchEncryptionError::SymmetricEncryptionError)?,
        })
    }

    /// Decrypt any plaintext that is deserializeable from bytes.
    pub fn decrypt<P: Plaintext>(&self, ciphertext: &SymmetricCiphertext) -> Result<P> {
        use aes_gcm::KeyInit as _; // putting this in the global scope causes Hmac<Sha256> to be
                                   // ambiguous for some reason

        let key: &Key<SymmetricCipher> = &self.0;
        let cipher = SymmetricCipher::new(key);
        let plaintext_bytes = cipher
            .decrypt(&ciphertext.nonce, ciphertext.ct_body.as_ref())
            .map_err(|_| BatchEncryptionError::SymmetricDecryptionError)?;
        Ok(bcs::from_bytes(&plaintext_bytes)
            .map_err(|_| BatchEncryptionError::DeserializationError)?)
    }
}

pub fn hmac_kdf(
    otp_source: impl AsRef<[u8]>,
) -> GenericArray<u8, <Sha256 as OutputSizeUser>::OutputSize> {
    // TODO should we use this instead? https://docs.rs/hkdf/latest/hkdf/
    let mut mac: Hmac<Sha256> = Hmac::new_from_slice(b"") // TODO should I put a key here?
        .expect("HMAC can take key of any size");
    // TODO should this be an option or result?
    mac.update(otp_source.as_ref());
    mac.finalize().into_bytes()
}

/// hash-2-curve for bn254. Taken from p. 23 of
/// https://wahby.net/bls-hash-ches19-talk.pdf
///
/// On the number of iterations: In bn254, the order of G1 is
/// approximately the same as the size of the field Fq. Every
/// x coordinate either maps to exactly two curve points (x, y)
/// and (x, -y), or to exactly zero curve points. So there are
/// |G2|/2 \approx |F_q|/2 possible x coordinates for the points
/// on G2. This means that each iteration of the below algorithm
/// has approximately probability 1/2 of succeeding on a random
/// input (modeling the hash as a random oracle), and thus the
/// probability of failure of this fn is 1/2^256 on a random
/// input.
pub fn hash_g2_element(g2_element: G2Affine) -> Result<G1Affine> {
    for ctr in 0..=u8::MAX {
        let mut hash_source_bytes = Vec::new();
        g2_element
            .serialize_compressed(&mut hash_source_bytes)
            .unwrap();
        let mut ctr_bytes = Vec::from([ctr]);
        hash_source_bytes.append(&mut ctr_bytes);
        println!("{:?}", hash_source_bytes);
        let field_hasher = <DefaultFieldHasher<Sha256> as HashToField<Fq>>::new(&[]);
        let [x]: [Fq; 1] = field_hasher.hash_to_field::<1>(&hash_source_bytes);
        println!("{:?}", x);

        // Rust does not optimise away addition with zero
        use crate::group::G1Config;
        let mut x3b = G1Config::add_b(x.square() * x);
        if !G1Config::COEFF_A.is_zero() {
            x3b += G1Config::mul_by_a(x);
        };

        // TODO vary the sign of y??
        if let Some(x3b_sqrt) = x3b.sqrt() {
            println!("{:?}", x3b_sqrt);
            let p = G1Affine::new_unchecked(x, x3b_sqrt).clear_cofactor();
            assert!(p.is_in_correct_subgroup_assuming_on_curve());
            println!("{:?}", p);
            return Ok(p);
        }
    }

    Err(BatchEncryptionError::Hash2CurveFailure)?
}

#[cfg(test)]
mod tests {
    use super::{OneTimePad, SymmetricCiphertext, SymmetricKey};
    use crate::{
        group::{Fq, Fr},
        shared::symmetric::{hmac_kdf, SymmetricCipher},
    };
    use aes_gcm::{aead::Aead as _, Key};
    use ark_ff::field_hashers::{DefaultFieldHasher, HashToField};
    use ark_std::rand::{thread_rng, RngCore as _};
    use generic_array::arr;
    use sha2::Sha256;

    #[test]
    fn test_ts_aes() {
        use aes_gcm::KeyInit as _; // putting this in the global scope causes Hmac<Sha256> to be
                                   // ambiguous for some reason

        let plaintext: [u8; 32] = [
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
            1, 1, 1,
        ];

        let key: Key<SymmetricCipher> =
            arr![u8; 192, 100, 19, 236, 152, 76,  83, 184, 195, 112, 203, 217, 182, 132, 106, 221];

        let cipher = SymmetricCipher::new(&key);

        let nonce = arr![u8; 38, 206, 151, 149, 191, 191,  99,  53, 160, 117, 249, 127];

        let ct = cipher.encrypt(&nonce, plaintext.as_ref()).unwrap();
        let expected_ct = vec![
            207, 143, 106, 246, 175, 96, 243, 179, 223, 186, 123, 69, 248, 37, 150, 207, 147, 67,
            253, 3, 229, 208, 112, 117, 180, 161, 219, 62, 136, 37, 60, 190, 108, 29, 101, 243, 86,
            31, 175, 230, 176, 229, 21, 117, 227, 234, 240, 234,
        ];

        println!("{:?}", expected_ct);

        assert_eq!(ct, expected_ct);
    }

    #[test]
    fn test_deserialize_symmetric_key() {
        let bytes = [
            153u8, 84, 154, 103, 123, 42, 86, 32, 99, 221, 55, 28, 130, 239, 154, 55,
        ];

        let key: SymmetricKey = bcs::from_bytes(&bytes).unwrap();

        println!("{:?}", key);
    }

    #[test]
    fn test_deserialize_symmetric_ciphertext() {
        let bytes = [
            142, 15, 186, 246, 119, 15, 171, 88, 56, 250, 102, 190, 19, 113, 77, 167, 52, 104, 52,
            185, 248, 5, 122, 58, 21, 118, 29, 130, 80, 78, 8, 142,
        ];

        let key = SymmetricKey(arr![u8;
            98, 146, 152, 254, 219,
            237,  33,  19,  55, 133,
            59, 155, 122, 211, 196,
            102
        ]);

        let ciphertext: SymmetricCiphertext = bcs::from_bytes(&bytes).unwrap();

        let plaintext: String = key.decrypt(&ciphertext).unwrap();

        println!("{:?}", ciphertext);
        println!("{:?}", plaintext);
    }

    #[test]
    fn test_symmetric_encrypt_decrypt() {
        let mut rng = thread_rng();

        let plaintext = String::from("hi");

        let key = SymmetricKey::new(&mut rng);

        let ct = key.encrypt(&mut rng, &plaintext).unwrap();

        let decrypted_plaintext: String = key.decrypt(&ct).unwrap();

        assert_eq!(decrypted_plaintext, plaintext);
    }

    #[test]
    fn test_otp() {
        let mut rng = thread_rng();
        let mut otp_source_bytes = [0; 256];
        rng.fill_bytes(&mut otp_source_bytes);
        let mut otp_source_bytes2 = [0; 256];
        rng.fill_bytes(&mut otp_source_bytes2);
        let otp = OneTimePad::from_source_bytes(otp_source_bytes);
        let otp2 = OneTimePad::from_source_bytes(otp_source_bytes2);
        let symmetric_key = SymmetricKey::new(&mut rng);
        let padded_key = otp.pad_key(&symmetric_key);
        let padded_key2 = otp2.pad_key(&symmetric_key);

        assert_eq!(symmetric_key, otp.unpad_key(&padded_key));
        assert_ne!(padded_key, padded_key2);
    }

    #[test]
    fn test_hmac_kdf() {
        println!("{:?}", hmac_kdf([1u8]));
    }

    #[test]
    fn test_hash_to_field() {
        let fr_hasher = <DefaultFieldHasher<Sha256> as HashToField<Fr>>::new(&[]);
        let x1: Fr = fr_hasher.hash_to_field::<1>(&[1])[0];
        let x2: Fr = fr_hasher.hash_to_field::<1>(&[1, 1])[0];
        println!("{:?}", x1);
        println!("{:?}", x2);
        let fq_hasher = <DefaultFieldHasher<Sha256> as HashToField<Fq>>::new(&[]);
        let x3: Fq = fq_hasher.hash_to_field::<1>(&[1])[0];
        let x4: Fq = fq_hasher.hash_to_field::<1>(&[1, 1])[0];
        println!("{:?}", x3);
        println!("{:?}", x4);
    }
}
