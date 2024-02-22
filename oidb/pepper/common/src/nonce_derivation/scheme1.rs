use crate::nonce_derivation::NonceDerivationScheme;
use serde::{Deserialize, Serialize};
use sha3::Digest;

pub struct Scheme {}

#[derive(Serialize, Deserialize)]
pub struct PreImage {
    pub epk: Vec<u8>,
    pub expiry_time_sec: u64,
    pub blinder: Vec<u8>,
}

impl NonceDerivationScheme for Scheme {
    type PreImage = PreImage;

    fn derive_nonce(pre_image: &Self::PreImage) -> Vec<u8> {
        let mut hasher = sha3::Sha3_256::new();
        hasher.update(DST);
        let hash_input = bcs::to_bytes(pre_image).unwrap();
        hasher.update(hash_input);
        hasher.finalize().to_vec()
    }
}

static DST: &[u8] = b"APTOS_OIDB_NONCE_DERIVATION_SCHEME1";
