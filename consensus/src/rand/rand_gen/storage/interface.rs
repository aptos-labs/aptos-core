// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::rand::rand_gen::types::{
    AugData, AugDataId, CertifiedAugData, RandDecision, RandShare, ShareId,
};
use aptos_consensus_types::randomness::RandMetadata;

pub trait RandStorage<S, P> {
    fn save_share(&self, share: &RandShare<S>) -> anyhow::Result<()>;
    fn save_decision(&self, decision: &RandDecision<P>) -> anyhow::Result<()>;

    fn get_all_shares(&self) -> anyhow::Result<Vec<(ShareId, RandShare<S>)>>;
    fn get_all_decision(&self) -> anyhow::Result<Vec<(RandMetadata, RandDecision<P>)>>;

    fn remove_shares(&self, shares: impl Iterator<Item = RandShare<S>>) -> anyhow::Result<()>;
    fn remove_decisions(
        &self,
        decisions: impl Iterator<Item = RandDecision<P>>,
    ) -> anyhow::Result<()>;
}

pub trait AugDataStorage<D> {
    fn save_aug_data(&self, aug_data: &AugData<D>) -> anyhow::Result<()>;
    fn save_certified_aug_data(
        &self,
        certified_aug_data: &CertifiedAugData<D>,
    ) -> anyhow::Result<()>;

    fn get_all_aug_data(&self) -> anyhow::Result<Vec<(AugDataId, AugData<D>)>>;
    fn get_all_certified_aug_data(&self) -> anyhow::Result<Vec<(AugDataId, CertifiedAugData<D>)>>;

    fn remove_aug_data(&self, aug_data: impl Iterator<Item = AugData<D>>) -> anyhow::Result<()>;
    fn remove_certified_aug_data(
        &self,
        certified_aug_data: impl Iterator<Item = CertifiedAugData<D>>,
    ) -> anyhow::Result<()>;
}
