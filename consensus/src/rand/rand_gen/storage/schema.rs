// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::rand::rand_gen::types::{AugData, AugDataId, CertifiedAugData, TAugmentedData};
use velor_schemadb::{
    define_schema,
    schema::{KeyCodec, Schema, ValueCodec},
    ColumnFamilyName,
};
use std::marker::PhantomData;

pub(crate) const KEY_PAIR_CF_NAME: ColumnFamilyName = "key_pair";

define_schema!(KeyPairSchema, (), (u64, Vec<u8>), KEY_PAIR_CF_NAME);

impl KeyCodec<KeyPairSchema> for () {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl ValueCodec<KeyPairSchema> for (u64, Vec<u8>) {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

pub(crate) const AUG_DATA_CF_NAME: ColumnFamilyName = "aug_data";
#[derive(Debug)]
pub struct AugDataSchema<D>(PhantomData<D>);

impl<D: TAugmentedData> Schema for AugDataSchema<D> {
    type Key = AugDataId;
    type Value = AugData<D>;

    const COLUMN_FAMILY_NAME: ColumnFamilyName = AUG_DATA_CF_NAME;
}

impl<D: TAugmentedData> KeyCodec<AugDataSchema<D>> for AugDataId {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl<D: TAugmentedData> ValueCodec<AugDataSchema<D>> for AugData<D> {
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

impl<D: TAugmentedData> Schema for CertifiedAugDataSchema<D> {
    type Key = AugDataId;
    type Value = CertifiedAugData<D>;

    const COLUMN_FAMILY_NAME: ColumnFamilyName = CERTIFIED_AUG_DATA_CF_NAME;
}

impl<D: TAugmentedData> KeyCodec<CertifiedAugDataSchema<D>> for AugDataId {
    fn encode_key(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(self)?)
    }

    fn decode_key(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}

impl<D: TAugmentedData> ValueCodec<CertifiedAugDataSchema<D>> for CertifiedAugData<D> {
    fn encode_value(&self) -> anyhow::Result<Vec<u8>> {
        Ok(bcs::to_bytes(&self)?)
    }

    fn decode_value(data: &[u8]) -> anyhow::Result<Self> {
        Ok(bcs::from_bytes(data)?)
    }
}
