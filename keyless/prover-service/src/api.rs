// Copyright © Aptos Foundation

// Copyright © Aptos Foundation

use aptos_crypto::ed25519::Ed25519Signature;
use aptos_types::{
    keyless::{Groth16Zkp, Pepper},
    transaction::authenticator::EphemeralPublicKey,
};
use ark_bn254::{self, Fr};
use ark_ff::{BigInteger, PrimeField};
use serde::{Deserialize, Serialize};

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
        training_wheels_signature: Ed25519Signature,
    },
    Error {
        message: String,
    },
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
