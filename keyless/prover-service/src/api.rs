use std::collections::HashMap;

use serde::{Serialize, Deserialize};
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey, Ed25519Signature};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{
    keyless::{Groth16Zkp, Pepper},
    transaction::authenticator::EphemeralPublicKey
};

use serde_json::value::Value;
use rust_rapidsnark::FullProver;
use aptos_types::jwks::rsa::RSA_JWK;
use anyhow::{anyhow, Result};
use ark_ff::{PrimeField, BigInteger};
use ark_bn254::{self, Fr};
use aptos_types::keyless::Groth16ZkpAndStatement;

use crate::{metrics, input_conversion::rsa::RsaPublicKey};

//#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
//pub struct EphemeralPublicKeyBlinder(pub(crate) Vec<u8>);

pub type EphemeralPublicKeyBlinder = Vec<u8>;

// TODO can I wrap this in a struct while preserving serialization format?
pub type PoseidonHash = [u8; 32];


// TODO move to encoding.rs?
pub trait AsFr {
    fn as_fr(&self) -> Fr;
}

pub trait FromFr {
    fn from_fr(fr: &Fr) -> Self;
}

impl AsFr for PoseidonHash {
    fn as_fr(&self) -> Fr {
        Fr::from_le_bytes_mod_order(self.as_slice())
    }
}

impl FromFr for PoseidonHash {
    fn from_fr(fr: &Fr) -> Self {
        fr.into_bigint().to_bytes_le().try_into().unwrap()
    }
}

impl AsFr for EphemeralPublicKeyBlinder {
    fn as_fr(&self) -> Fr {
        Fr::from_le_bytes_mod_order(self)
    }
}

impl FromFr for EphemeralPublicKeyBlinder {
    fn from_fr(fr: &Fr) -> Self {
        fr.into_bigint().to_bytes_le()
    }
}

impl AsFr for Pepper {
    fn as_fr(&self) -> Fr {
        Fr::from_le_bytes_mod_order(self.to_bytes())
    }
}





#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProverServerResponse {
    Success {
        proof: Groth16Zkp,   
        #[serde(with = "hex")]
        public_inputs_hash: PoseidonHash,  
        training_wheels_signature: Ed25519Signature
    },
    Error {
        message: String
    }
}

#[derive(Debug)]
pub struct Input {
    pub jwt_b64: String,
    pub epk: EphemeralPublicKey,
    pub epk_blinder_fr: Fr,
    pub exp_date_secs: u64,
    pub pepper_fr: Fr,
    pub variable_keys: HashMap<String, String>,
    pub exp_horizon_secs: u64,
    pub use_extra_field: bool,
    // TODO add jwk field 
    // TODO jwk_b64 -> jwt_parts
}



#[derive(Debug, Serialize, Deserialize)]
pub struct RequestInput {
    pub jwt_b64: String,
    pub epk: EphemeralPublicKey,
    #[serde(with = "hex")]
    pub epk_blinder: EphemeralPublicKeyBlinder,
    pub exp_date_secs: u64,
    pub exp_horizon_secs: u64,
    pub pepper: Pepper,
    pub uid_key: String,
    pub extra_field: Option<String>,
    pub aud_override: Option<String>, 
}


impl RequestInput {
    pub fn decode(self) -> Result<Input, anyhow::Error> {
        if let Some(_) = self.aud_override {
            Err(anyhow!("aud_override is unsupported for now"))
        } else {
            let extra_field_jwt_key = match &self.extra_field { Some(x) => String::from(x), None => String::from("") };

            Ok(Input {
                jwt_b64: self.jwt_b64,
                epk: self.epk,
                epk_blinder_fr: self.epk_blinder.as_fr(),
                exp_date_secs: self.exp_date_secs,
                pepper_fr: self.pepper.as_fr(),
                variable_keys: HashMap::from([
                                             (String::from("uid"), self.uid_key),
                                             (String::from("extra"), extra_field_jwt_key),
                ]),
                exp_horizon_secs: self.exp_horizon_secs,
                use_extra_field: match self.extra_field { Some(_) => true, None => false }
            })
        }
    }

}
