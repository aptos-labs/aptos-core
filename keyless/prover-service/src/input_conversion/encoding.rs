use aptos_types::jwks::rsa::RSA_JWK;
use aptos_types::transaction::authenticator::EphemeralPublicKey;
use ark_bls12_381::Fr;
use ark_bls12_381::FrConfig;
use ark_ff::Fp256;
use ark_ff::MontBackend;
use num_bigint::BigUint;
use anyhow::Result;
use anyhow::anyhow;
use ark_ff::{PrimeField, Fp};
use serde::{Serialize, Deserialize};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};


pub type RsaSignature = BigUint;



pub trait FromB64 {
    fn from_b64(s: &str) -> Result<Self> where Self: Sized;
}



impl FromB64 for RsaSignature {
    /// JWT signature is encoded in big-endian.
    fn from_b64(s: &str) -> Result<Self> {
        Ok(BigUint::from_bytes_be(&URL_SAFE_NO_PAD.decode(s)?))
    }
}


pub trait FromHex {
    fn from_hex(s: &str) -> Result<Self> where Self: Sized;
}

impl FromHex for EphemeralPublicKey {
    fn from_hex(s: &str) -> Result<Self> where Self: Sized {
        Ok(EphemeralPublicKey::try_from(hex::decode(s)?.as_slice())?)
    }
}


pub struct JwtParts {
    header: String,
    payload: String,
    signature: String,
}

#[derive(Serialize, Deserialize)]
pub struct JwtHeader {
    pub kid: String,
}

#[derive(Serialize, Deserialize)]
pub struct JwtPayload {
    pub iss: String,
}

impl FromB64 for JwtParts {
    fn from_b64(s: &str) -> Result<Self> where Self: Sized {
        let jwt_parts: Vec<&str> = s.split(".").collect();
        Ok(Self {
            header: String::from(*jwt_parts.get(0).ok_or(anyhow!("JWT did not parse correctly"))?),
            payload: String::from(*jwt_parts.get(1).ok_or(anyhow!("JWT did not parse correctly"))?),
            signature: String::from(*jwt_parts.get(2).ok_or(anyhow!("JWT did not parse correctly"))?),
        })
    }
}

impl JwtParts {

    pub fn unsigned_undecoded(&self) -> String {
        String::from(&self.header) + "." + &self.payload
    }

    pub fn payload_undecoded(&self) -> String {
        String::from(&self.payload)
    }

    pub fn header_undecoded_with_dot(&self) -> String {
        String::from(&self.header) + "."
    }

    pub fn header_decoded(&self) -> Result<String> {
        Ok(String::from_utf8(URL_SAFE_NO_PAD.decode(&self.header)?)?)
    }

    pub fn payload_decoded(&self) -> Result<String> {
        Ok(String::from_utf8(URL_SAFE_NO_PAD.decode(&self.payload)?)?)
    }

    pub fn signature(&self) -> Result<RsaSignature> {
        RsaSignature::from_b64(&self.signature)
    }
}



pub struct UnsignedJwtPartsWithPadding {
    b: Vec<u8>
}


impl UnsignedJwtPartsWithPadding {
    pub fn from_b64_bytes_with_padding(b: &[u8]) -> Self {
        Self { b: Vec::from(b) }
    }


    pub fn payload_with_padding(&self) -> Result<Vec<u8>> {
        let first_dot = self.b
                            .iter()
                            .position(|c| c == &('.' as u8)).ok_or(anyhow!("Not a valid jwt; has no \".\""))?;

        Ok(Vec::from( &self.b[first_dot+1..]))
    }
}








/// Trait which signals that this type allows conversion into 64-bit limbs. Used for JWT signature
/// and JWK modulus.
pub trait As64BitLimbs {
    fn as_64bit_limbs(&self) -> Vec<u64>;
}

impl As64BitLimbs for RSA_JWK {
    fn as_64bit_limbs(&self) -> Vec<u64> {
        let modulus_bytes = URL_SAFE_NO_PAD.decode(&self.n).expect("JWK should always have a properly-encoded modulus");
        // JWKs encode modulus in big-endian order
        let modulus_biguint : BigUint = BigUint::from_bytes_be(&modulus_bytes);
        modulus_biguint.to_u64_digits()
    }
}

impl As64BitLimbs for RsaSignature {
    fn as_64bit_limbs(&self) -> Vec<u64> {
        self.to_u64_digits()
    }
}
