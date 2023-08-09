// Copyright © Aptos Foundation

use move_core_types::ident_str;
use move_core_types::identifier::IdentStr;
use move_core_types::move_resource::MoveStructType;
use crate::validator_info::ValidatorInfo;
use serde::{Serialize, Deserialize};
use anyhow::Result;
use aptos_dkg::pvss::{das, WeightedTranscript, traits::Transcript};

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

impl DKGTranscriptWrapper {
    pub fn verify(
        &self,
        dkg_pvss_config: &DKGPvssConfig,
    ) -> anyhow::Result<()> {
        self.trx_one_third.verify(&dkg_pvss_config.wc_1, &dkg_pvss_config.pp, &dkg_pvss_config.eks, dkg_pvss_config.dst)?;
        self.trx_two_third.verify(&dkg_pvss_config.wc_2, &dkg_pvss_config.pp, &dkg_pvss_config.eks, dkg_pvss_config.dst)?;
        Ok(())
    }

    pub fn aggregate_with(
        &mut self,
        dkg_pvss_config: &DKGPvssConfig,
        other: &Self
    ){
        self.trx_one_third.aggregate_with(&dkg_pvss_config.wc_1, &other.trx_one_third);
        self.trx_two_third.aggregate_with(&dkg_pvss_config.wc_2, &other.trx_two_third);
    }
}
