//! copied from https://github.com/arkworks-rs/algebra/issues/178#issuecomment-1413219278
//!
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress, Validate};

pub(crate) fn ark_se<S, A: CanonicalSerialize>(a: &A, s: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
      let mut bytes = vec![];
      a.serialize_with_mode(&mut bytes, Compress::Yes).map_err(serde::ser::Error::custom)?;
      s.serialize_bytes(&bytes)
}

pub(crate) fn ark_de<'de, D, A: CanonicalDeserialize>(data: D) -> Result<A, D::Error> where D: serde::de::Deserializer<'de> {
      let s: Vec<u8> = serde::de::Deserialize::deserialize(data)?;
      let a = A::deserialize_with_mode(s.as_slice(), Compress::Yes, Validate::Yes);
      a.map_err(serde::de::Error::custom)
}


