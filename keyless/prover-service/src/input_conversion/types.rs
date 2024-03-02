use std::collections::HashMap;
use std::ops::{Add, AddAssign};

use anyhow::{Error, anyhow};
use aptos_types::transaction::authenticator::EphemeralPublicKey;
use ark_bn254::Fr;

use super::bits::Bits;



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






/// Type for "ascii" byte representation during conversion.
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Ascii {
    pub(crate) bytes: Vec<u8>
}


impl Ascii {
    pub fn new() -> Self {
        Ascii { bytes: Vec::new() }
    }
    
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Ascii { bytes: Vec::from(bytes) }
    }


    pub fn push(&mut self, c: u8) {
        self.bytes.push(c);
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn as_bytes<'a>(&'a self) -> &'a[u8] {
        self.bytes.as_slice()
    }

    pub fn pad(self, max_size : usize) -> Result<Self, anyhow::Error> {
        let mut bytes = self.bytes;
        if max_size < bytes.len() {
            Err(anyhow!("max_size exceeded: {} is too long for max size {}", String::from_utf8(Vec::from(bytes)).unwrap(), max_size))
        } else {
            bytes.extend([0].repeat(max_size-bytes.len()));
            Ok(Self { bytes })
        }
    }

    /// Note: this panics on invalid utf-8. 
    pub fn find(&self, s: &str) -> Option<usize> {
        String::from_utf8(self.bytes.clone()).expect("Should always decode valid utf-8").find(&s)

    }
    
    /// Note: this panics on invalid utf-8. 
    pub fn find_starting_at(&self, i: usize, s: &str) -> Option<usize> {
        Some(String::from_utf8(self.bytes.clone())
             .expect("Should always decode valid utf-8")[i..]
             .find(&s)?
             + i
            )
    }

    pub fn first_non_space_char_starting_at(&self, i: usize) -> Option<usize> {
        let mut pos = i;
        while pos < self.bytes.len() && self.bytes[pos] == (' ' as u8) {
            pos += 1;
        }
        if pos < self.bytes.len() { Some(pos) } else { None }
    }

    pub fn value_starting_at(&self, i: usize) -> Option<(String, usize)> {
        if self.bytes[i] == ('"' as u8) {
            // handle quoted values
            let mut pos = i+1;
            while pos < self.bytes.len() && self.bytes[pos] != ('"' as u8) {
                pos += 1;
            }
            if pos < self.bytes.len() { 
                Some(
                    (
                        String::from_utf8(
                            Vec::from(
                                &self.bytes[i+1..pos]))
                        .expect("Should always decode valid utf-8"),
                        pos
                    ))
            } else { None }
        } else {
            // handle unquoted values
            let mut pos = i;
            while pos < self.bytes.len() 
                && self.bytes[pos] != (' ' as u8) 
                && self.bytes[pos] != (',' as u8) 
                && self.bytes[pos] != ('}' as u8) {
                pos += 1;
            }
            if pos < self.bytes.len() { 
                Some(
                    (
                        String::from_utf8(
                            Vec::from(
                                &self.bytes[i..pos]))
                        .expect("Should always decode valid utf-8"),
                        pos-1
                    ))
            } else { None }
        }
    }

    pub fn whole_field(&self, start: usize, value_end: usize) -> Option<String> {
        let next_non_space = self.first_non_space_char_starting_at(value_end+1)?;
        if self.bytes[next_non_space] != (',' as u8) &&
           self.bytes[next_non_space] != ('}' as u8) {
            None
        } else {
            Some(
                String::from_utf8(
                    Vec::from(
                        &self.bytes[start..next_non_space+1]
                        ))
                .expect("Should always decode valid utf-8"))
        }
    }


    pub fn header_with_dot(&self) -> Result<Ascii, Error> {
        let first_dot = self.bytes
                            .iter()
                            .position(|c| c == &('.' as u8)).ok_or(anyhow!("Not a valid jwt; has no \".\""))?;

        Ok(Ascii { bytes: Vec::from( &self.bytes[..first_dot+1] ) })
    }

    pub fn payload(&self) -> Result<Ascii, Error> {
        let first_dot = self.bytes
                            .iter()
                            .position(|c| c == &('.' as u8)).ok_or(anyhow!("Not a valid jwt; has no \".\""))?;

        Ok(Ascii { bytes: Vec::from( &self.bytes[first_dot+1..] ) })
    }
}

impl From<&str> for Ascii {
    fn from(bytes: &str) -> Self {
        Ascii { bytes: Vec::from(bytes) }
    }
}


impl TryFrom<Bits> for Ascii {
    type Error = anyhow::Error; //put something sane here

    /// Input: Bits in BIG-ENDIAN order
    /// Output: Ascii bytes in BIG_ENDIAN order
    fn try_from(value: Bits) -> Result<Self, Self::Error> {
        if value.b.len() % 8 != 0 {
            Err(anyhow!("Tried to convert bits to bytes, where bit length is not divisible by 8"))
        } else {
            let mut bytes  = Vec::new();

            for i in 0..(value.b.len()/8) {
                let idx = i*8;
                let bits_for_chunk : &str = &value[idx..idx+8];
                let chunk_byte = u8::from_str_radix(bits_for_chunk, 2).expect("Binary string should parse");

                bytes.push(chunk_byte);
            }

            Ok(Ascii { bytes })
        }
    }
}

impl AddAssign<Ascii> for Ascii {
    fn add_assign(&mut self, rhs: Ascii) {
        self.bytes.extend(rhs.bytes);
    }
}

impl Add<Ascii> for Ascii {
    type Output = Ascii;

    fn add(self, rhs: Ascii) -> Self::Output {
        let mut bytes = self.bytes.clone();
        bytes.extend(rhs.bytes);
        Ascii { bytes }
    }
}









#[cfg(test)]
mod tests {
    use crate::input_conversion::types::Ascii;


    #[test]
    fn test_ascii_find() {
        let a1 = Ascii::from("test test");
        assert!(a1.find("test") == Some(0));
        let a2 = Ascii::from("offset test test");
        assert!(a2.find("test") == Some(7));
        let a3 = Ascii::from("est");
        assert!(a3.find("test") == None);
    }

    #[test]
    fn test_ascii_find_starting_at() {
        let a1 = Ascii::from("test test");
        assert!(a1.find_starting_at(1, "test") == Some(5));
        let a2 = Ascii::from("test test");
        assert!(a2.find_starting_at(5, "test") == Some(5));
        let a3 = Ascii::from("test test");
        assert!(a3.find_starting_at(6, "test") == None);
    }
}
