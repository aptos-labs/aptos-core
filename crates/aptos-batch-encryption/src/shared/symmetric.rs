use aes_gcm::{aead::Aead as _, aes::Aes128, AeadCore, Aes128Gcm, Aes256Gcm, AesGcm, Key, KeySizeUser, Nonce};
use ark_bn254::Bn254;
use serde::{Deserialize, Serialize};
use crate::group::{G2Affine, Config};
use ark_serialize::CanonicalSerialize as _;
use hmac::{Hmac, Mac};
use ark_ec::{bn::BnConfig, short_weierstrass::SWCurveConfig, hashing::{curve_maps::wb::WBMap, map_to_curve_hasher::MapToCurveBasedHasher, HashToCurve}, AffineRepr as _};
use ark_ff::{Field, field_hashers::{DefaultFieldHasher, HashToField}};
use rand_core::{CryptoRng, RngCore};
use ark_std::Zero;
use crate::{errors::BatchEncryptionError, group::{G1Affine, G1Config, G1Projective, Fq}, traits::Plaintext};
use sha2::{digest::{consts::{B0, B1, U16}, generic_array::{functional::FunctionalSequence as _, sequence::Split, GenericArray}, typenum::{UInt, UTerm}, OutputSizeUser}, Sha256};
use anyhow::Result;



type KeySize = <Aes128 as KeySizeUser>::KeySize;
type SymmetricCipher = Aes128Gcm;
type SymmetricNonce = Nonce<<AesGcm<Aes128, UInt<UInt<UInt<UInt<UTerm, B1>, B1>, B0>, B0>> as AeadCore>::NonceSize>;


#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SymmetricKey(GenericArray<u8, KeySize>);
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OneTimePad(GenericArray<u8, KeySize>);
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OneTimePaddedKey(GenericArray<u8, KeySize>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymmetricCiphertext {
    nonce: SymmetricNonce,
    ct_body: Vec<u8>,
}

impl OneTimePad {
    /// Take some source bytes that are high-entropy (but not necessarily uniformly-distributed),
    /// and generate a one-time pad of [`KeySize`] length that is indistingushable from uniform
    /// random.
    pub fn from_source_bytes(otp_source: impl AsRef<[u8]>) -> Self {
        let otp = hmac_kdf(otp_source);
        let (otp_first_half, _) : (GenericArray<u8, U16>, GenericArray<u8, U16>) = otp.split();
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
        Self(Aes128Gcm::generate_key(rng))
    }


    /// Encrypt any plaintext that is serializable into bytes.
    pub fn encrypt<R: RngCore + CryptoRng>(&self, rng: &mut R, plaintext: &impl Plaintext) -> Result<SymmetricCiphertext> {
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
            ct_body: cipher.encrypt(&nonce, plaintext_bytes.as_ref()).map_err(|_| BatchEncryptionError::SymmetricEncryptionError)?
        })
    }

    /// Decrypt any plaintext that is deserializeable from bytes.
    pub fn decrypt<'a, P: Plaintext>(&self, ciphertext: &SymmetricCiphertext) -> Result<P> {
        use aes_gcm::KeyInit as _; // putting this in the global scope causes Hmac<Sha256> to be
                                   // ambiguous for some reason
    
        let key: &Key<SymmetricCipher> = &self.0;
        let cipher = SymmetricCipher::new(key);
        let plaintext_bytes = cipher.decrypt(&ciphertext.nonce, ciphertext.ct_body.as_ref()).map_err(|_| BatchEncryptionError::SymmetricDecryptionError)?;
        Ok(bcs::from_bytes(&plaintext_bytes).map_err(|_| BatchEncryptionError::DeserializationError)?)
    }
}



fn hmac_kdf(otp_source: impl AsRef<[u8]>) -> GenericArray<u8, <Sha256 as OutputSizeUser>::OutputSize> {
    let mut mac : Hmac<Sha256> = Hmac::new_from_slice(b"") // TODO should I put a key here?
        .expect("HMAC can take key of any size");
    // TODO should this be an option or result?
    mac.update(otp_source.as_ref());
    mac.finalize().into_bytes()
}


/// hash-2-curve for bn254. Taken from p. 23 of 
/// https://wahby.net/bls-hash-ches19-talk.pdf
pub fn hash_g2_element(g2_element: G2Affine) -> Result<G1Affine>
{
    for ctr in 0..u64::MAX {
        let mut hash_source_bytes = Vec::new();
        g2_element.serialize_compressed(&mut hash_source_bytes).unwrap();
        let mut ctr_bytes = Vec::from(ctr.to_be_bytes());
        hash_source_bytes.append(&mut ctr_bytes); 
        let field_hasher = <DefaultFieldHasher<Sha256> as HashToField<Fq>>::new(&[]);
        let [x] : [Fq; 1] = field_hasher.hash_to_field::<1>(&hash_source_bytes);

        // Rust does not optimise away addition with zero
        let mut x3b = <ark_bn254::Config as BnConfig>::G1Config::add_b(x.square() * x);
        if !<ark_bn254::Config as BnConfig>::G1Config::COEFF_A.is_zero() {
            x3b += <ark_bn254::Config as BnConfig>::G1Config::mul_by_a(x);
        };

        if let Some(x3b_sqrt) = x3b.sqrt() {
            return Ok(G1Affine::new(
                x, 
                x3b_sqrt
            ))
        }
    }

    Err(BatchEncryptionError::Hash2CurveFailure)?
}

#[cfg(test)]
mod tests {
    use ark_std::rand::thread_rng;
    use rand_core::RngCore;
    use crate::traits::Plaintext;
    use super::{OneTimePad, SymmetricKey};



    #[test]
    fn test_symmetric_encrypt_decrypt() {
        let mut rng = thread_rng();
        
        let plaintext = String::from("hi");

        let key = SymmetricKey::new(&mut rng);

        let ct = key.encrypt(&mut rng, &plaintext).unwrap();

        let decrypted_plaintext : String = key.decrypt(&ct).unwrap();

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

}
