// Copyright © Aptos Foundation

use std::io::{Read, Write};
use crate::validator_info::ValidatorInfo;
use anyhow::Result;
use aptos_dkg::pvss::{das, traits::Transcript, WeightedTranscript};
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
    pub dst: &'static [u8],
}

impl DKGPvssConfig {
    pub fn new(
        wc_1: <WT as Transcript>::SecretSharingConfig,
        wc_2: <WT as Transcript>::SecretSharingConfig,
        pp: <das::Transcript as Transcript>::PvssPublicParameters,
        eks: Vec<<das::Transcript as Transcript>::EncryptPubKey>,
        dst: &'static [u8],
    ) -> Self {
        Self {
            wc_1,
            wc_2,
            pp,
            eks,
            dst,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct DKGTranscriptWrapper {
    pub trx_one_third: WT,
    pub trx_two_third: WT,
}

impl TryFrom<&[u8]> for DKGTranscriptWrapper {
    type Error = CryptoMaterialError;

    fn try_from(value: &[u8]) -> std::result::Result<Self, Self::Error> {
        //TODO: make it compile
        // let mut cursor = std::io::Cursor::new(value);
        // let trx1_len = cursor.read_u64::<ByteOrder>()? as usize;
        // let mut buf1: Vec<u8> = vec![0; trx1_len];
        // cursor.read_exact(buf1.as_mut_slice())?;
        // let trx1: WT = buf1.try_into()?;
        // let trx2_len = cursor.read_u64()? as usize;
        // let mut buf2: Vec<u8> = vec![0; trx2_len];
        // cursor.read_exact(buf2.as_mut_slice())?;
        // let trx2: WT = buf1.try_into()?;
        // Ok(Self {
        //     trx_one_third: trx1,
        //     trx_two_third: trx2,
        // })
        todo!()
    }
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

    pub fn to_bytes(&self) -> Vec<u8> {
        //TODO: make it compile
        // let trx1_bytes = self.trx_one_third.to_bytes();
        // let trx2_bytes = self.trx_two_third.to_bytes();
        // let mut buffer: Vec<u8> = Vec::with_capacity(16 + trx1_bytes.len() + trx2_bytes.len());
        // let mut cursor = std::io::Cursor::new(&mut buffer);
        // cursor.write_u64(trx1_bytes.len() as u64).unwrap();
        // cursor.write_all(trx1_bytes.as_slice()).unwrap();
        // cursor.write_u64(trx2_bytes.len() as u64).unwrap();
        // cursor.write_all(trx2_bytes.as_slice()).unwrap();
        // buffer
        todo!()
    }
}
