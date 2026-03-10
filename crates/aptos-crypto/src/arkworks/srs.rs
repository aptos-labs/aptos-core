// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Structured Reference String (SRS) utilities.
//!
//! This module defines data structures and helpers for working with
//! Structured Reference Strings (SRS) used in pairing-based and
//! polynomial-commitment–style cryptographic protocols.

use crate::utils;
use ark_ec::CurveGroup;
use ark_ff::Field;
use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, Read, SerializationError, Valid, Validate,
    Write,
};

/// Represents the type of Structured Reference String (SRS) basis.
///
/// This enum is a lightweight discriminator for *selecting* which SRS construction to use.
pub enum SrsType {
    /// The SRS should use a Lagrange basis.
    Lagrange,
    /// The SRS should use a Powers-of-Tau basis.
    PowersOfTau,
}

/// A concrete representation of a Structured Reference String (SRS).
///
/// This enum stores the actual group elements defining an SRS, parameterized
/// by an affine curve representation. Each variant corresponds to a different
/// basis commonly used in e.g. polynomial commitment schemes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SrsBasis<C: CurveGroup> {
    /// The SRS is represented in the Lagrange basis.
    Lagrange {
        /// The vector `[G·ℓ_0(τ), G·ℓ_1(τ), G·ℓ_2(τ), …]`, where `G` is a fixed generator.
        lagr: Vec<C::Affine>,
    },

    /// The SRS is represented in the Powers-of-Tau basis.
    PowersOfTau {
        /// The vector `[G, G·τ, G·τ², …]`, where `G` is a fixed generator.
        tau_powers: Vec<C::Affine>,
    },
}

// Enums need to be (de)serialised manually
impl<C: CurveGroup> CanonicalSerialize for SrsBasis<C> {
    fn serialize_with_mode<W: Write>(
        &self,
        mut writer: W,
        compress: Compress,
    ) -> Result<(), SerializationError> {
        match self {
            SrsBasis::Lagrange { lagr: lagr_g1 } => {
                0u8.serialize_with_mode(&mut writer, compress)?; // variant tag
                lagr_g1.serialize_with_mode(&mut writer, compress)?;
            },
            SrsBasis::PowersOfTau {
                tau_powers: tau_powers_g1,
            } => {
                1u8.serialize_with_mode(&mut writer, compress)?; // variant tag
                tau_powers_g1.serialize_with_mode(&mut writer, compress)?;
            },
        }
        Ok(())
    }

    fn serialized_size(&self, compress: Compress) -> usize {
        1 + match self {
            SrsBasis::Lagrange { lagr: lagr_g1 } => lagr_g1.serialized_size(compress),
            SrsBasis::PowersOfTau {
                tau_powers: tau_powers_g1,
            } => tau_powers_g1.serialized_size(compress),
        }
    }
}

impl<C: CurveGroup> Valid for SrsBasis<C> {
    fn check(&self) -> Result<(), SerializationError> {
        match self {
            SrsBasis::Lagrange { lagr: lagr_g1 } => {
                for g in lagr_g1 {
                    g.check()?;
                }
            },
            SrsBasis::PowersOfTau {
                tau_powers: tau_powers_g1,
            } => {
                for g in tau_powers_g1 {
                    g.check()?;
                }
            },
        }
        Ok(())
    }
}

impl<C: CurveGroup> CanonicalDeserialize for SrsBasis<C> {
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        compress: Compress,
        validate: Validate,
    ) -> Result<Self, SerializationError> {
        // Read the variant tag first
        let tag = u8::deserialize_with_mode(&mut reader, compress, validate)?;

        match tag {
            0 => {
                // Lagrange variant
                let lagr =
                    Vec::<C::Affine>::deserialize_with_mode(&mut reader, compress, validate)?;
                Ok(SrsBasis::Lagrange { lagr })
            },
            1 => {
                // Powers-of-Tau variant
                let tau_powers =
                    Vec::<C::Affine>::deserialize_with_mode(&mut reader, compress, validate)?;
                Ok(SrsBasis::PowersOfTau { tau_powers })
            },
            _ => Err(SerializationError::InvalidData),
        }
    }
}

/// Constructs a Structured Reference String (SRS) in the Lagrange basis.
///
/// This function generates the sequence:
/// `[G·ℓ_0(τ), G·ℓ_1(τ), G·ℓ_2(τ), …, G·ℓ_{n - 1}(τ)]`,
/// returning the result in affine form.
#[allow(non_snake_case)]
pub fn lagrange_basis<C: CurveGroup>(
    G: C,
    tau: C::ScalarField,
    n: usize,
    eval_dom: Radix2EvaluationDomain<C::ScalarField>,
) -> Vec<C::Affine> {
    let powers_of_tau = utils::powers(tau, n);
    let lagr_basis_scalars = eval_dom.ifft(&powers_of_tau);
    debug_assert!(lagr_basis_scalars.iter().sum::<C::ScalarField>() == C::ScalarField::ONE);

    G.batch_mul(&lagr_basis_scalars)
}

/// Constructs a Structured Reference String (SRS) in the Powers-of-Tau basis.
///
/// This function generates the sequence:
/// `[G, G·τ, G·τ², …, G·τ^(n - 1)]`,
/// returning the result in affine form.
#[allow(non_snake_case)]
pub fn powers_of_tau<C: CurveGroup>(G: C, tau: C::ScalarField, n: usize) -> Vec<C::Affine> {
    // We have to work over `CurveGroup` instead of `AffineRepr` here and in the above function `lagrange_basis()`
    // because for some reason only the former has `batch_mul()` implemented for its elements, and this is much
    // faster than doing the naive approach:
    //
    // let mut proj = Vec::with_capacity(n);
    // proj.push(base.into_group());
    // for i in 0..(n - 1) {
    //     proj.push(proj[i] * tau);
    // }
    // A::Group::normalize_batch(&proj)

    let powers_of_tau = utils::powers(tau, n);

    G.batch_mul(&powers_of_tau)
}
