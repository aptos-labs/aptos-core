// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::rand::rand_gen::{
    storage::interface::{AugDataStorage, RandStorage},
    types::{
        AugData, AugDataId, AugmentedData, CertifiedAugData, Proof, RandDecision, RandShare, Share,
        ShareId,
    },
};
use aptos_consensus_types::randomness::RandMetadata;
use aptos_infallible::RwLock;
use std::collections::HashMap;

pub struct InMemRandDb<S, P, D> {
    shares: RwLock<HashMap<ShareId, RandShare<S>>>,
    decisions: RwLock<HashMap<RandMetadata, RandDecision<P>>>,
    aug_data: RwLock<HashMap<AugDataId, AugData<D>>>,
    certified_aug_data: RwLock<HashMap<AugDataId, CertifiedAugData<D>>>,
}

impl<S, P, D> InMemRandDb<S, P, D> {
    pub fn new() -> Self {
        Self {
            shares: RwLock::new(HashMap::new()),
            decisions: RwLock::new(HashMap::new()),
            aug_data: RwLock::new(HashMap::new()),
            certified_aug_data: RwLock::new(HashMap::new()),
        }
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> RandStorage<S, P> for InMemRandDb<S, P, D> {
    fn save_share(&self, share: &RandShare<S>) -> anyhow::Result<()> {
        self.shares
            .write()
            .insert(share.share_id().clone(), share.clone());
        Ok(())
    }

    fn save_decision(&self, decision: &RandDecision<P>) -> anyhow::Result<()> {
        self.decisions
            .write()
            .insert(decision.rand_metadata().clone(), decision.clone());
        Ok(())
    }

    fn get_all_shares(&self) -> anyhow::Result<Vec<(ShareId, RandShare<S>)>> {
        Ok(self.shares.read().clone().into_iter().collect())
    }

    fn get_all_decisions(&self) -> anyhow::Result<Vec<(RandMetadata, RandDecision<P>)>> {
        Ok(self.decisions.read().clone().into_iter().collect())
    }

    fn remove_shares(&self, shares: impl Iterator<Item = RandShare<S>>) -> anyhow::Result<()> {
        for share in shares {
            self.shares.write().remove(&share.share_id());
        }
        Ok(())
    }

    fn remove_decisions(
        &self,
        decisions: impl Iterator<Item = RandDecision<P>>,
    ) -> anyhow::Result<()> {
        for decision in decisions {
            self.decisions.write().remove(decision.rand_metadata());
        }
        Ok(())
    }
}

impl<S: Share, P: Proof<Share = S>, D: AugmentedData> AugDataStorage<D> for InMemRandDb<S, P, D> {
    fn save_aug_data(&self, aug_data: &AugData<D>) -> anyhow::Result<()> {
        self.aug_data
            .write()
            .insert(aug_data.id(), aug_data.clone());
        Ok(())
    }

    fn save_certified_aug_data(
        &self,
        certified_aug_data: &CertifiedAugData<D>,
    ) -> anyhow::Result<()> {
        self.certified_aug_data
            .write()
            .insert(certified_aug_data.id(), certified_aug_data.clone());
        Ok(())
    }

    fn get_all_aug_data(&self) -> anyhow::Result<Vec<(AugDataId, AugData<D>)>> {
        Ok(self.aug_data.read().clone().into_iter().collect())
    }

    fn get_all_certified_aug_data(&self) -> anyhow::Result<Vec<(AugDataId, CertifiedAugData<D>)>> {
        Ok(self.certified_aug_data.read().clone().into_iter().collect())
    }

    fn remove_aug_data(&self, aug_data: impl Iterator<Item = AugData<D>>) -> anyhow::Result<()> {
        for data in aug_data {
            self.aug_data.write().remove(&data.id());
        }
        Ok(())
    }

    fn remove_certified_aug_data(
        &self,
        certified_aug_data: impl Iterator<Item = CertifiedAugData<D>>,
    ) -> anyhow::Result<()> {
        for data in certified_aug_data {
            self.certified_aug_data.write().remove(&data.id());
        }
        Ok(())
    }
}
