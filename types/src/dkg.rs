// Copyright © Aptos Foundation

use crate::validator_verifier::ValidatorVerifier;
use anyhow::Result;
use aptos_dkg::pvss::{das, traits::Transcript, WeightedTranscript};
use move_core_types::{ident_str, identifier::IdentStr, move_resource::MoveStructType};
use serde::{Deserialize, Serialize};
use aptos_crypto::ValidCryptoMaterial;
use crate::on_chain_config::ValidatorSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StartDKGEvent {
    pub target_epoch: u64,
    pub target_validator_set: ValidatorSet,
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
    pub epoch: u64,
    pub wc_1: <WT as Transcript>::SecretSharingConfig,
    pub wc_2: <WT as Transcript>::SecretSharingConfig,
    pub pp: <WT as Transcript>::PublicParameters,
    pub eks: Vec<<DT as Transcript>::EncryptPubKey>,
}

impl DKGPvssConfig {
    pub fn new(
        epoch: u64,
        wc_1: <WT as Transcript>::SecretSharingConfig,
        wc_2: <WT as Transcript>::SecretSharingConfig,
        pp: <das::Transcript as Transcript>::PublicParameters,
        eks: Vec<<das::Transcript as Transcript>::EncryptPubKey>,
    ) -> Self {
        Self {
            epoch,
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
    pub fn verify(&self, dkg_pvss_config: &DKGPvssConfig, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        let dealers = self.verify_dealers(verifier.len())?;

        let all_eks = dkg_pvss_config.eks.clone();

        let addresses = verifier.get_ordered_account_addresses();
        let dealers_addresses = dealers.iter().filter_map(|&pos| addresses.get(pos)).cloned().collect::<Vec<_>>();

        let spks = dealers_addresses.iter().filter_map(|author| verifier.get_public_key(author)).collect::<Vec<_>>();

        let aux = dealers_addresses.iter().map(|address| (dkg_pvss_config.epoch, address)).collect::<Vec<_>>();

        self.trx_one_third.verify(
            &dkg_pvss_config.wc_1,
            &dkg_pvss_config.pp,
            &spks,
            &all_eks,
            &aux,
        )?;
        self.trx_two_third.verify(
            &dkg_pvss_config.wc_2,
            &dkg_pvss_config.pp,
            &spks,
            &all_eks,
            &aux,
        )?;

        Ok(())
    }

    pub fn verify_dealers(&self, n: usize) -> anyhow::Result<Vec<usize>> {
        let dealers_1 = self.trx_one_third.get_dealers().iter().map(|player| player.id).collect::<Vec<usize>>();
        let dealers_2 = self.trx_two_third.get_dealers().iter().map(|player| player.id).collect::<Vec<usize>>();
        if dealers_1 != dealers_2 {
            anyhow::bail!("[DKG] trx dealers mismatch!");
        }
        if dealers_1.iter().any(|id| *id >= n) {
            anyhow::bail!("[DKG] trx dealers out of range!");
        }
        Ok(dealers_1)
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
