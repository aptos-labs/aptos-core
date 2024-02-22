/// Implement this to define how to get a pepper pre-image.
/// A pepper pre-image is a byte string that will be mapped to the final pepper by a VUF.
pub trait PepperPreImageDerivation {
    type Source;
    fn derive(src: &Self::Source) -> Vec<u8>;
}

/// A pepper per-image derivation scheme where:
/// the pepper pre-image is the hash of a naive but canonical concatenation of (iss, sub, aud).
///
/// TODO: better name?
pub mod scheme0;

/// A pepper per-image derivation scheme where:
/// the pepper pre-image is the hash of the bcs serialization of (iss, uid_key, uid_val, aud).
///
/// TODO: better name?
pub mod scheme1;
