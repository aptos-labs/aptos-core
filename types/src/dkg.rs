// Copyright © Aptos Foundation

use crate::validator_info::ValidatorInfo;
use anyhow::Result;
use aptos_dkg::pvss::{das, traits::Transcript, WeightedTranscript};
use move_core_types::{ident_str, identifier::IdentStr, move_resource::MoveStructType};
use serde::{Deserialize, Serialize};
use aptos_crypto::ValidCryptoMaterial;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartDKGEvent {
    pub target_epoch: u64,
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

type DT = das::Transcript;
type WT = WeightedTranscript<DT>;

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct DKGPvssConfig {
    pub wc_1: <WT as Transcript>::SecretSharingConfig,
    pub wc_2: <WT as Transcript>::SecretSharingConfig,
    pub pp: <WT as Transcript>::PvssPublicParameters,
    pub eks: Vec<<DT as Transcript>::EncryptPubKey>,
}

impl DKGPvssConfig {
    pub fn new(
        wc_1: <WT as Transcript>::SecretSharingConfig,
        wc_2: <WT as Transcript>::SecretSharingConfig,
        pp: <das::Transcript as Transcript>::PvssPublicParameters,
        eks: Vec<<das::Transcript as Transcript>::EncryptPubKey>,
    ) -> Self {
        Self {
            wc_1,
            wc_2,
            pp,
            eks,
        }
    }

    pub fn num_bytes(&self) -> usize {
        // dkg todo: compute size
        0
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
        )?;
        self.trx_two_third.verify(
            &dkg_pvss_config.wc_2,
            &dkg_pvss_config.pp,
            &dkg_pvss_config.eks,
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
