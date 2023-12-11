// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::rand::rand_gen::types::{
    AugData, AugDataId, AugmentedData, CertifiedAugData, Proof, RandDecision, RandShare, Share,
    ShareId,
};
use aptos_consensus_types::randomness::RandMetadata;
use aptos_schemadb::{
    schema::{KeyCodec, Schema, ValueCodec},
    ColumnFamilyName,
};
use std::marker::PhantomData;

pub(crate) const SHARE_CF_NAME: ColumnFamilyName = "rand_share";
#[derive(Debug)]
pub struct RandShareSchema<S>(PhantomData<S>);

impl<S: Share> Schema for RandShareSchema<S> {
    type Key = ShareId;
    type Value = RandShare<S>;

    const COLUMN_FAMILY_NAME: ColumnFamilyName = SHARE_CF_NAME;
}

impl<S: Share> KeyCodec<RandShareSchema<S>> for ShareId {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl<S: Share> ValueCodec<RandShareSchema<S>> for RandShare<S> {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

pub(crate) const DECISION_CF_NAME: ColumnFamilyName = "rand_decision";
#[derive(Debug)]
pub struct RandDecisionSchema<P>(PhantomData<P>);

impl<P: Proof> Schema for RandDecisionSchema<P> {
    type Key = RandMetadata;
    type Value = RandDecision<P>;

    const COLUMN_FAMILY_NAME: ColumnFamilyName = DECISION_CF_NAME;
}

impl<P: Proof> KeyCodec<RandDecisionSchema<P>> for RandMetadata {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl<P: Proof> ValueCodec<RandDecisionSchema<P>> for RandDecision<P> {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

pub(crate) const AUG_DATA_CF_NAME: ColumnFamilyName = "aug_data";
#[derive(Debug)]
pub struct AugDataSchema<D>(PhantomData<D>);

impl<D: AugmentedData> Schema for AugDataSchema<D> {
    type Key = AugDataId;
    type Value = AugData<D>;

    const COLUMN_FAMILY_NAME: ColumnFamilyName = AUG_DATA_CF_NAME;
}

impl<D: AugmentedData> KeyCodec<AugDataSchema<D>> for AugDataId {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl<D: AugmentedData> ValueCodec<AugDataSchema<D>> for AugData<D> {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

pub(crate) const CERTIFIED_AUG_DATA_CF_NAME: ColumnFamilyName = "certified_aug_data";
#[derive(Debug)]
pub struct CertifiedAugDataSchema<D>(PhantomData<D>);

impl<D: AugmentedData> Schema for CertifiedAugDataSchema<D> {
    type Key = AugDataId;
    type Value = CertifiedAugData<D>;

    const COLUMN_FAMILY_NAME: ColumnFamilyName = CERTIFIED_AUG_DATA_CF_NAME;
}

impl<D: AugmentedData> KeyCodec<CertifiedAugDataSchema<D>> for AugDataId {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl<D: AugmentedData> ValueCodec<CertifiedAugDataSchema<D>> for CertifiedAugData<D> {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}
