/// Implement this to define how to derive the nonce that will be signed by OIDC provider.
pub trait NonceDerivationScheme {
    type PreImage;
    fn derive_nonce(pre_image: &Self::PreImage) -> Vec<u8>;
}

/// A nonce derivation scheme where nonce is the hash of
/// the bcs serialization of `(epk, expiry time, blinder)`.
///
/// TODO: better name?
pub mod scheme1;

// pub mod poseidon_bn254;
