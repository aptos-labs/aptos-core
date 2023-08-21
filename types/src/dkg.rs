// Copyright © Aptos Foundation

use std::io::{Read, Write};
use crate::validator_info::ValidatorInfo;
use anyhow::Result;
use byteorder::{LittleEndian, WriteBytesExt};
use aptos_dkg::pvss::{das, Player, traits::Transcript, WeightedTranscript};
use move_core_types::{ident_str, identifier::IdentStr, move_resource::MoveStructType};
use serde::{Deserialize, Serialize};
use aptos_crypto::{CryptoMaterialError, ValidCryptoMaterial};
use crate::on_chain_config::{ConfigID, OnChainConfig};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartDKGEvent {
    pub locked_new_validator_info: Vec<ValidatorInfo>,
}

impl StartDKGEvent {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }
}
impl MoveStructType for StartDKGEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("dkg");
    const STRUCT_NAME: &'static IdentStr = ident_str!("StartDKGEvent");
}

type WT = WeightedTranscript<das::Transcript>;

#[derive(Clone)]
pub struct DKGPvssConfig {
    pub wc_1: <WT as Transcript>::SecretSharingConfig,
    pub wc_2: <WT as Transcript>::SecretSharingConfig,
    pub pp: <das::Transcript as Transcript>::PvssPublicParameters,
    pub eks: Vec<<das::Transcript as Transcript>::EncryptPubKey>,
    pub my_index: Player,
    pub dst: &'static [u8],
}

impl DKGPvssConfig {
    pub fn new(
        wc_1: <WT as Transcript>::SecretSharingConfig,
        wc_2: <WT as Transcript>::SecretSharingConfig,
        pp: <das::Transcript as Transcript>::PvssPublicParameters,
        eks: Vec<<das::Transcript as Transcript>::EncryptPubKey>,
        my_index: Player,
        dst: &'static [u8],
    ) -> Self {
        Self {
            wc_1,
            wc_2,
            pp,
            eks,
            my_index,
            dst,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct DKGTranscriptWrapper {
    pub trx_one_third: WT,
    pub trx_two_third: WT,
}

impl DKGTranscriptWrapper {
    pub fn verify(&self, dkg_pvss_config: &DKGPvssConfig) -> anyhow::Result<()> {
        self.trx_one_third.verify(
            &dkg_pvss_config.wc_1,
            &dkg_pvss_config.pp,
            &dkg_pvss_config.eks,
            dkg_pvss_config.dst,
        )?;
        self.trx_two_third.verify(
            &dkg_pvss_config.wc_2,
            &dkg_pvss_config.pp,
            &dkg_pvss_config.eks,
            dkg_pvss_config.dst,
        )?;
        Ok(())
    }

    pub fn aggregate_with(&mut self, dkg_pvss_config: &DKGPvssConfig, other: &Self) {
        self.trx_one_third
            .aggregate_with(&dkg_pvss_config.wc_1, &other.trx_one_third);
        self.trx_two_third
            .aggregate_with(&dkg_pvss_config.wc_2, &other.trx_two_third);
    }

    pub fn num_bytes(&self) -> usize {
        self.trx_one_third.to_bytes().len() + self.trx_two_third.to_bytes().len()
    }
}
