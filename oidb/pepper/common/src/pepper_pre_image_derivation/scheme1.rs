use crate::pepper_pre_image_derivation::PepperPreImageDerivation;
use serde::{Deserialize, Serialize};

pub struct Scheme {}

#[derive(Serialize, Deserialize)]
pub struct Source {
    pub iss: String,
    pub uid_key: String,
    pub uid_val: String,
    pub aud: String,
}

impl PepperPreImageDerivation for Scheme {
    type Source = Source;

    fn derive(src: &Self::Source) -> Vec<u8> {
        bcs::to_bytes(src).unwrap()
    }
}
