//! Fiat-Shamir Random Generator
use ark_ff::PrimeField;
use ark_serialize::CanonicalSerialize;
use ark_std::{rand::RngCore, vec::Vec};
use blake2::{Blake2b512, Digest};

/// RNG that reads challenges from a Merlin transcript. Used to bind sumcheck
/// round challenges to the main protocol transcript.
pub struct TranscriptRng<'a, F: PrimeField> {
    transcript: &'a mut merlin::Transcript,
    _phantom: core::marker::PhantomData<F>,
}

impl<'a, F: PrimeField> TranscriptRng<'a, F> {
    /// Create a new TranscriptRng. Use this instead of `FeedableRNG::setup()`.
    pub fn new(transcript: &'a mut merlin::Transcript) -> Self {
        Self {
            transcript,
            _phantom: core::marker::PhantomData,
        }
    }
}

impl<F: PrimeField> RngCore for TranscriptRng<'_, F> {
    fn next_u32(&mut self) -> u32 {
        let mut buf = [0u8; 4];
        self.fill_bytes(&mut buf);
        u32::from_le_bytes(buf)
    }

    fn next_u64(&mut self) -> u64 {
        let mut buf = [0u8; 8];
        self.fill_bytes(&mut buf);
        u64::from_le_bytes(buf)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.transcript.challenge_bytes(b"sumcheck_round", dest);
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), ark_std::rand::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

impl<F: PrimeField> FeedableRNG for TranscriptRng<'_, F> {
    type Error = super::Error;

    fn setup() -> Self {
        panic!("TranscriptRng must be created with TranscriptRng::new(transcript)");
    }

    fn feed<M: CanonicalSerialize>(&mut self, msg: &M) -> Result<(), Self::Error> {
        let mut buf = Vec::new();
        msg.serialize_compressed(&mut buf)
            .map_err(|_| super::Error::SerializationError)?;
        self.transcript.append_message(b"sumcheck_prover_msg", &buf);
        Ok(())
    }
}

/// Random Field Element Generator where randomness `feed` adds entropy for the output.
///
/// Implementation should support all types of input that has `ToBytes` trait.
///
/// Same sequence of `feed` and `get` call should yield same result!
pub trait FeedableRNG: RngCore {
    /// Error type
    type Error: ark_std::error::Error + From<super::Error>;
    /// Setup should not have any parameter.
    fn setup() -> Self;

    /// Provide randomness for the generator, given the message.
    fn feed<M: CanonicalSerialize>(&mut self, msg: &M) -> Result<(), Self::Error>;
}

/// 512-bits digest hash pseudorandom generator
pub struct Blake2b512Rng {
    /// current digest instance
    current_digest: Blake2b512,
}

impl FeedableRNG for Blake2b512Rng {
    type Error = super::Error;

    fn setup() -> Self {
        Self {
            current_digest: Blake2b512::new(),
        }
    }

    fn feed<M: CanonicalSerialize>(&mut self, msg: &M) -> Result<(), Self::Error> {
        let mut buf = Vec::new();
        msg.serialize_uncompressed(&mut buf)?;
        self.current_digest.update(&buf);
        Ok(())
    }
}

impl RngCore for Blake2b512Rng {
    fn next_u32(&mut self) -> u32 {
        let mut temp = [0u8; 4];
        self.fill_bytes(&mut temp);
        u32::from_le_bytes(temp)
    }

    fn next_u64(&mut self) -> u64 {
        let mut temp = [0u8; 8];
        self.fill_bytes(&mut temp);
        u64::from_le_bytes(temp)
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.try_fill_bytes(dest).unwrap()
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), ark_std::rand::Error> {
        let mut digest = self.current_digest.clone();
        let mut output = digest.finalize();
        let output_size = Blake2b512::output_size();
        let mut ptr = 0;
        let mut digest_ptr = 0;
        while ptr < dest.len() {
            dest[ptr] = output[digest_ptr];
            ptr += 1usize;
            digest_ptr += 1;
            if digest_ptr == output_size {
                self.current_digest.update(output);
                digest = self.current_digest.clone();
                output = digest.finalize();
                digest_ptr = 0;
            }
        }
        self.current_digest.update(output);
        Ok(())
    }
}
