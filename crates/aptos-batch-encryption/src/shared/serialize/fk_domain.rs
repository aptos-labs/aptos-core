// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use ark_ff::FftField;
use ark_poly::{domain::DomainCoeff, EvaluationDomain, Radix2EvaluationDomain};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    ser::SerializeStruct as _,
    Deserialize, Serialize,
};
use std::{fmt, marker::PhantomData};
use crate::shared::algebra::fk_algorithm::*;

impl<F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize> Serialize
    for FKDomain<F, T>
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("FKDomain", 3)?;
        state.serialize_field(
            "toeplitz_domain_dimension",
            &self.toeplitz_domain.dimension(),
        )?;
        state.serialize_field("fft_domain_size", &self.fft_domain.size)?;
        state.serialize_field("prepared_toeplitz_inputs", &self.prepared_toeplitz_inputs)?;
        state.end()
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "snake_case")]
enum Field {
    ToeplitzDomainDimension,
    FftDomainSize,
    PreparedToeplitzInputs,
}

struct FKDomainVisitor<F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize> {
    _phantom: PhantomData<F>,
    _phantom2: PhantomData<T>,
}

impl<'de, F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize> Visitor<'de>
    for FKDomainVisitor<F, T>
{
    type Value = FKDomain<F, T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("struct FKDomain")
    }

    fn visit_seq<V>(self, mut seq: V) -> Result<FKDomain<F, T>, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let toeplitz_domain_dimension = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(0, &self))?;
        let fft_domain_size = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(1, &self))?;
        let prepared_toeplitz_inputs = seq
            .next_element()?
            .ok_or_else(|| de::Error::invalid_length(2, &self))?;
        Ok(FKDomain {
            toeplitz_domain: ToeplitzDomain::new(toeplitz_domain_dimension)
                .ok_or(de::Error::custom("Toeplitz domain initialization failed"))?,
            fft_domain: Radix2EvaluationDomain::new(fft_domain_size).ok_or(de::Error::custom(
                "Radix2EvaluationDomain initialization failed",
            ))?,
            prepared_toeplitz_inputs,
        })
    }

    fn visit_map<V>(self, mut map: V) -> Result<FKDomain<F, T>, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut toeplitz_domain_dimension: Option<usize> = None;
        let mut fft_domain_size: Option<usize> = None;
        let mut prepared_toeplitz_inputs: Option<Vec<PreparedInput<F, T>>> = None;
        while let Some(key) = map.next_key()? {
            match key {
                Field::ToeplitzDomainDimension => {
                    if toeplitz_domain_dimension.is_some() {
                        return Err(de::Error::duplicate_field("toeplitz_domain_dimension"));
                    }
                    toeplitz_domain_dimension = Some(map.next_value()?);
                },
                Field::FftDomainSize => {
                    if fft_domain_size.is_some() {
                        return Err(de::Error::duplicate_field("fft_domain_size"));
                    }
                    fft_domain_size = Some(map.next_value()?);
                },
                Field::PreparedToeplitzInputs => {
                    if prepared_toeplitz_inputs.is_some() {
                        return Err(de::Error::duplicate_field("prepared_toeplitz_inputs"));
                    }
                    prepared_toeplitz_inputs = Some(map.next_value()?);
                },
            }
        }
        let toeplitz_domain_dimension = toeplitz_domain_dimension
            .ok_or_else(|| de::Error::missing_field("toeplitz_domain_dimension"))?;
        let fft_domain_size =
            fft_domain_size.ok_or_else(|| de::Error::missing_field("fft_domain_size"))?;
        let prepared_toeplitz_inputs = prepared_toeplitz_inputs
            .ok_or_else(|| de::Error::missing_field("prepared_toeplitz_inputs"))?;
        Ok(FKDomain {
            toeplitz_domain: ToeplitzDomain::new(toeplitz_domain_dimension)
                .ok_or(de::Error::custom("Toeplitz domain initialization failed"))?,
            fft_domain: Radix2EvaluationDomain::new(fft_domain_size).ok_or(de::Error::custom(
                "Radix2EvaluationDomain initialization failed",
            ))?,
            prepared_toeplitz_inputs,
        })
    }
}

impl<'de, F: FftField, T: DomainCoeff<F> + CanonicalSerialize + CanonicalDeserialize>
    Deserialize<'de> for FKDomain<F, T>
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "toeplitz_domain_dimension",
            "fft_domain_size",
            "prepared_toeplitz_inputs",
        ];
        deserializer.deserialize_struct("FKDomain", FIELDS, FKDomainVisitor {
            _phantom: PhantomData,
            _phantom2: PhantomData,
        })
    }
}
