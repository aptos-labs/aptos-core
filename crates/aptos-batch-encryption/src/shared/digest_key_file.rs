// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    group::{Fr, G1Affine, G1Projective, G2Affine},
    shared::{
        algebra::fk_algorithm::{FKDomain, PreparedInput, ToeplitzDomain},
        digest::DigestKey,
    },
};
use anyhow::{bail, Result};
use aptos_crypto::arkworks::serialization::{ark_de, ark_se};
use ark_ec::{AffineRepr, PrimeGroup};
use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, SerializationError, Validate,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{Read, Seek, Write},
    path::Path,
};

// Structs

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum Header {
    V1(HeaderV1),
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct HeaderV1 {
    pub batch_size: usize,
    pub num_rounds: usize,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub tau_g2: G2Affine,
    pub toeplitz_domain: ToeplitzDomain<Fr>,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub fft_domain: Radix2EvaluationDomain<Fr>,
}

impl Header {
    pub fn representation_size_bytes() -> usize {
        bcs::to_bytes(&Self::V1(HeaderV1 {
            batch_size: 0,
            num_rounds: 0,
            tau_g2: G2Affine::generator(),
            toeplitz_domain: ToeplitzDomain::new(1).unwrap(),
            fft_domain: Radix2EvaluationDomain::new(1).unwrap(),
        }))
        .expect("Serialization should never fail")
        .len()
    }
}

impl HeaderV1 {
    pub fn num_powers_per_round(&self) -> usize {
        self.batch_size + 1
    }

    pub fn prepared_input_size(&self) -> usize {
        2 * self.batch_size
    }

    pub fn round_size_bytes(&self) -> usize {
        self.num_powers_per_round() * G1Affine::generator().serialized_size(Compress::No)
            + self.prepared_input_size() * G1Projective::generator().serialized_size(Compress::No)
    }
}

/// The point of this struct is that it's serialized into a bytestring of known length, so that we
/// can seek within the DigestKey file.
pub struct Round {
    pub tau_powers_g1: Vec<G1Affine>,
    pub prepared_toeplitz_input: PreparedInput<Fr, G1Projective>,
}

// Writing

impl From<(Vec<G1Affine>, PreparedInput<Fr, G1Projective>)> for Round {
    fn from(value: (Vec<G1Affine>, PreparedInput<Fr, G1Projective>)) -> Self {
        Self {
            tau_powers_g1: value.0,
            prepared_toeplitz_input: value.1,
        }
    }
}

impl From<DigestKey> for (HeaderV1, Vec<Round>) {
    fn from(value: DigestKey) -> Self {
        (
            HeaderV1 {
                batch_size: value.max_batch_size(),
                num_rounds: value.tau_powers_g1.len(),
                tau_g2: value.tau_g2,
                toeplitz_domain: value.fk_domain.toeplitz_domain,
                fft_domain: value.fk_domain.fft_domain,
            },
            value
                .tau_powers_g1
                .into_iter()
                .zip(value.fk_domain.prepared_toeplitz_inputs)
                .map(Round::from)
                .collect(),
        )
    }
}

pub fn write_digest_key(file: &Path, dk: DigestKey) -> Result<()> {
    let mut file = File::create(file)?;

    let (header_v1, rounds): (HeaderV1, Vec<Round>) = dk.into();

    let header = Header::V1(header_v1.clone());

    file.write_all(&bcs::to_bytes(&header)?)?;

    for round in rounds {
        write_round(&file, &round, &header_v1)?;
    }

    Ok(())
}

pub fn write_round(file: &File, round: &Round, header: &HeaderV1) -> Result<()> {
    if round.tau_powers_g1.len() != header.num_powers_per_round() {
        bail!(
            "Digest key shape mismatch: round has {} powers of tau, expected {}",
            round.tau_powers_g1.len(),
            header.num_powers_per_round()
        );
    } else if round.prepared_toeplitz_input.y.len() != header.prepared_input_size() {
        bail!(
            "Digest key shape mismatch: round has prepared input of size {}, expected {}",
            round.prepared_toeplitz_input.y.len(),
            header.prepared_input_size()
        );
    } else {
        for power in &round.tau_powers_g1 {
            power.serialize_uncompressed(file)?;
        }
        for elt in &round.prepared_toeplitz_input.y {
            elt.serialize_uncompressed(file)?;
        }
        Ok(())
    }
}

// Reading

impl From<(HeaderV1, Vec<Round>)> for DigestKey {
    // Assumes correct shape
    fn from(value: (HeaderV1, Vec<Round>)) -> Self {
        let (tau_powers_g1, prepared_toeplitz_inputs): (
            Vec<Vec<G1Affine>>,
            Vec<PreparedInput<Fr, G1Projective>>,
        ) = value
            .1
            .into_iter()
            .map(|round| (round.tau_powers_g1, round.prepared_toeplitz_input))
            .collect();

        Self {
            tau_g2: value.0.tau_g2,
            tau_powers_g1,
            fk_domain: FKDomain {
                toeplitz_domain: value.0.toeplitz_domain,
                fft_domain: value.0.fft_domain,
                prepared_toeplitz_inputs,
            },
        }
    }
}

pub fn read_digest_key(file: &Path) -> Result<DigestKey> {
    let mut file = File::open(file)?;

    let mut buf: Vec<u8> = vec![0; Header::representation_size_bytes()];
    file.read_exact(&mut buf)?;
    let header: Header = bcs::from_bytes(&buf)?;

    match header {
        Header::V1(header_v1) => read_digest_key_v1(&file, header_v1),
    }
}

pub fn read_digest_key_range(
    file: &Path,
    starting_round: usize,
    num_rounds_to_read: usize,
) -> Result<DigestKey> {
    let mut file = File::open(file)?;

    let mut buf: Vec<u8> = vec![0; Header::representation_size_bytes()];
    file.read_exact(&mut buf)?;
    let header: Header = bcs::from_bytes(&buf)?;

    match header {
        Header::V1(header_v1) => {
            read_digest_key_v1_range(&mut file, header_v1, starting_round, num_rounds_to_read)
        },
    }
}

pub fn read_digest_key_v1(file: &File, header: HeaderV1) -> Result<DigestKey> {
    let expected_size_bytes =
        Header::representation_size_bytes() + header.round_size_bytes() * header.num_rounds;

    if file.metadata()?.len() as usize != expected_size_bytes {
        bail!("File is of the incorrect size: expected {} rounds yielding {} bytes, but got {} bytes instead.",
            header.num_rounds,
            expected_size_bytes,
            file.metadata()?.len()
        );
    }

    let rounds: Vec<Round> = (0..header.num_rounds)
        .map(|_| read_round(file, &header))
        .collect::<Result<Vec<Round>>>()?;

    Ok(DigestKey::from((header, rounds)))
}

pub fn read_digest_key_v1_range(
    file: &mut File,
    header: HeaderV1,
    starting_round: usize,
    num_rounds_to_read: usize,
) -> Result<DigestKey> {
    let expected_size_bytes =
        Header::representation_size_bytes() + header.round_size_bytes() * header.num_rounds;

    if file.metadata()?.len() as usize != expected_size_bytes {
        bail!("File is of the incorrect size: expected {} rounds yielding {} bytes, but got {} bytes instead.",
            header.num_rounds,
            expected_size_bytes,
            file.metadata()?.len()
        );
    }
    if starting_round + num_rounds_to_read > header.num_rounds {
        bail!("Specified starting round {} and num rounds to read {}, but the digest key file only has {} rounds.",
            starting_round,
            num_rounds_to_read,
            header.num_rounds
        );
    }

    file.seek_relative((starting_round * header.round_size_bytes()) as i64)?;

    let rounds: Vec<Round> = (0..num_rounds_to_read)
        .map(|_| read_round(file, &header))
        .collect::<Result<Vec<Round>>>()?;

    Ok(DigestKey::from((header, rounds)))
}

pub fn read_round(file: &File, header: &HeaderV1) -> Result<Round> {
    let tau_powers_g1: Vec<G1Affine> = (0..header.num_powers_per_round())
        .map(|_| G1Affine::deserialize_with_mode(file, Compress::No, Validate::No))
        .collect::<std::result::Result<Vec<G1Affine>, SerializationError>>()?;

    let prepared_input_y: Vec<G1Projective> = (0..header.prepared_input_size())
        .map(|_| G1Projective::deserialize_with_mode(file, Compress::No, Validate::No))
        .collect::<std::result::Result<Vec<G1Projective>, SerializationError>>()?;

    Ok(Round {
        tau_powers_g1,
        prepared_toeplitz_input: PreparedInput::new(prepared_input_y),
    })
}

#[cfg(test)]
mod tests {
    use crate::shared::{digest::DigestKey, digest_key_file::*};
    use ark_std::rand::thread_rng;
    use tempfile::NamedTempFile;

    #[test]
    fn test_serialize_deserialize() {
        let mut rng = thread_rng();
        let dk = DigestKey::new(&mut rng, 8, 5).unwrap();

        let file = NamedTempFile::new().unwrap();
        write_digest_key(file.path(), dk.clone()).unwrap();

        let dk_from_file = read_digest_key(file.path()).unwrap();

        assert_eq!(dk, dk_from_file);
    }

    #[test]
    fn test_serialize_deserialize_range() {
        let mut rng = thread_rng();
        let dk = DigestKey::new(&mut rng, 8, 5).unwrap();

        let file = NamedTempFile::new().unwrap();
        write_digest_key(file.path(), dk.clone()).unwrap();

        let dk_from_file = read_digest_key(file.path()).unwrap();
        let dk_from_file_2 = read_digest_key_range(file.path(), 0, 5).unwrap();

        assert_eq!(dk_from_file, dk_from_file_2);
    }

    #[test]
    #[should_panic]
    fn test_serialize_deserialize_range_oob() {
        let mut rng = thread_rng();
        let dk = DigestKey::new(&mut rng, 8, 5).unwrap();

        let file = NamedTempFile::new().unwrap();
        write_digest_key(file.path(), dk.clone()).unwrap();

        let _ = read_digest_key_range(file.path(), 0, 6).unwrap();
    }

    #[test]
    #[should_panic]
    fn test_serialize_deserialize_range_oob_2() {
        let mut rng = thread_rng();
        let dk = DigestKey::new(&mut rng, 8, 5).unwrap();

        let file = NamedTempFile::new().unwrap();
        write_digest_key(file.path(), dk.clone()).unwrap();

        let _ = read_digest_key_range(file.path(), 1, 5).unwrap();
    }

    #[test]
    fn test_serialize_deserialize_subrange() {
        let mut rng = thread_rng();
        let mut dk = DigestKey::new(&mut rng, 8, 5).unwrap();

        let file = NamedTempFile::new().unwrap();
        write_digest_key(file.path(), dk.clone()).unwrap();

        let dk_from_file = read_digest_key_range(file.path(), 2, 3).unwrap();

        dk.tau_powers_g1.remove(0);
        dk.tau_powers_g1.remove(0);
        dk.fk_domain.prepared_toeplitz_inputs.remove(0);
        dk.fk_domain.prepared_toeplitz_inputs.remove(0);

        assert_eq!(dk, dk_from_file);
    }
}
