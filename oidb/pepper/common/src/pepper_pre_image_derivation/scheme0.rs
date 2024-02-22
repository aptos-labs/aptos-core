use crate::pepper_pre_image_derivation::PepperPreImageDerivation;
use byteorder::{BigEndian, WriteBytesExt};
use std::io::Write;

pub struct Scheme {}

pub struct Source {
    pub iss: String,
    pub sub: String,
    pub aud: String,
}

impl PepperPreImageDerivation for Scheme {
    type Source = Source;

    fn derive(src: &Self::Source) -> Vec<u8> {
        let mut ret = vec![];
        let issuer_bytes = src.iss.as_bytes();
        let sub_bytes = src.sub.as_bytes();
        let aud_bytes = src.aud.as_bytes();
        ret.write_u64::<BigEndian>(issuer_bytes.len() as u64)
            .unwrap();
        ret.write_all(issuer_bytes).unwrap();
        ret.write_u64::<BigEndian>(sub_bytes.len() as u64).unwrap();
        ret.write_all(sub_bytes).unwrap();
        ret.write_u64::<BigEndian>(aud_bytes.len() as u64).unwrap();
        ret.write_all(aud_bytes).unwrap();
        ret
    }
}
