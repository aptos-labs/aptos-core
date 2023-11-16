// Copyright © Aptos Foundation

use crate::validator_verifier::ValidatorVerifier;
use anyhow::Result;
use aptos_dkg::pvss::{self, traits::Transcript, WeightedTranscript};
use move_core_types::{ident_str, identifier::IdentStr, move_resource::MoveStructType};
use serde::{Deserialize, Serialize};
use aptos_crypto::ValidCryptoMaterial;
use crate::on_chain_config::ValidatorSet;

pub type Trx = pvss::das::Transcript;
pub type WTrx = WeightedTranscript<Trx>;
pub type DkgPP = <WTrx as Transcript>::PublicParameters;
pub type SSConfig = <WTrx as Transcript>::SecretSharingConfig;
pub type EncPK = <WTrx as Transcript>::EncryptPubKey;

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

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct DKGPvssConfig {
    pub epoch: u64,
    // weighted config for randomness fallback path
    pub wc_f: SSConfig,
    // weighted config for randomness optimistic path
    pub wc_o: SSConfig,
    // DKG public parameters
    pub pp: DkgPP,
    // DKG encryption public keys
    pub eks: Vec<EncPK>,
}

impl DKGPvssConfig {
    pub fn new(
        epoch: u64,
        wc_f: SSConfig,
        wc_o: SSConfig,
        pp: DkgPP,
        eks: Vec<EncPK>,
    ) -> Self {
        Self {
            epoch,
            wc_f,
            wc_o,
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
    // DKG weighted transcript for randomness fallback path
    pub trx_f: WTrx,
    // DKG weighted transcript for randomness optimistic path
    pub trx_o: WTrx,
}

impl DKGTranscriptWrapper {
    pub fn verify(&self, dkg_pvss_config: &DKGPvssConfig, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        let dealers = self.verify_dealers(verifier.len())?;

        let all_eks = dkg_pvss_config.eks.clone();

        let addresses = verifier.get_ordered_account_addresses();
        let dealers_addresses = dealers.iter().filter_map(|&pos| addresses.get(pos)).cloned().collect::<Vec<_>>();

        let spks = dealers_addresses.iter().filter_map(|author| verifier.get_public_key(author)).collect::<Vec<_>>();

        let aux = dealers_addresses.iter().map(|address| (dkg_pvss_config.epoch, address)).collect::<Vec<_>>();

        self.trx_f.verify(
            &dkg_pvss_config.wc_f,
            &dkg_pvss_config.pp,
            &spks,
            &all_eks,
            &aux,
        )?;
        self.trx_o.verify(
            &dkg_pvss_config.wc_o,
            &dkg_pvss_config.pp,
            &spks,
            &all_eks,
            &aux,
        )?;

        Ok(())
    }

    pub fn verify_dealers(&self, n: usize) -> anyhow::Result<Vec<usize>> {
        let dealers_f = self.trx_f.get_dealers().iter().map(|player| player.id).collect::<Vec<usize>>();
        let dealers_o = self.trx_o.get_dealers().iter().map(|player| player.id).collect::<Vec<usize>>();
        if dealers_f != dealers_o {
            anyhow::bail!("[DKG] trx dealers mismatch!");
        }
        if dealers_f.iter().any(|id| *id >= n) {
            anyhow::bail!("[DKG] trx dealers out of range!");
        }
        Ok(dealers_f)
    }

    pub fn aggregate_with(&mut self, dkg_pvss_config: &DKGPvssConfig, other: &Self) {
        self.trx_f
            .aggregate_with(&dkg_pvss_config.wc_f, &other.trx_f);
        self.trx_o
            .aggregate_with(&dkg_pvss_config.wc_o, &other.trx_o);
    }

    pub fn num_bytes(&self) -> usize {
        self.trx_f.to_bytes().len() + self.trx_o.to_bytes().len()
    }
}
