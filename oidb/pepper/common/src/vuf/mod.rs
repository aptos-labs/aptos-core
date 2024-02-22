use anyhow::Result;
use ark_std::rand::{CryptoRng, RngCore};

/// Implement this to define a VUF (verifiable unpredictable function).
pub trait VUF {
    fn scheme_name() -> String;

    /// Return `(sk, pk)`.
    fn setup<R: CryptoRng + RngCore>(rng: &mut R) -> (Vec<u8>, Vec<u8>);

    fn pk_from_sk(sk: &[u8]) -> Result<Vec<u8>>;

    /// Return `(output, proof)`.
    fn eval(sk: &[u8], input: &[u8]) -> Result<(Vec<u8>, Vec<u8>)>;

    fn verify(pk: &[u8], input: &[u8], output: &[u8], proof: &[u8]) -> Result<()>;
}

/// a BLS VUF where:
/// - The underlying curve is BLS12-381.
/// - Input/output is in G1 and public key is in G2.
///
/// TODO: better name?
pub mod scheme0;
