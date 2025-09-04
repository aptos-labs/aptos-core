// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::rand::rand_gen::types::{AugData, AugDataId, CertifiedAugData};

pub trait RandStorage<D>: Send + Sync + 'static {
    fn save_key_pair_bytes(&self, epoch: u64, key_pair: Vec<u8>) -> anyhow::Result<()>;
    fn save_aug_data(&self, aug_data: &AugData<D>) -> anyhow::Result<()>;
    fn save_certified_aug_data(
        &self,
        certified_aug_data: &CertifiedAugData<D>,
    ) -> anyhow::Result<()>;

    fn get_key_pair_bytes(&self) -> anyhow::Result<Option<(u64, Vec<u8>)>>;
    fn get_all_aug_data(&self) -> anyhow::Result<Vec<(AugDataId, AugData<D>)>>;
    fn get_all_certified_aug_data(&self) -> anyhow::Result<Vec<(AugDataId, CertifiedAugData<D>)>>;

    fn remove_aug_data(&self, aug_data: Vec<AugData<D>>) -> anyhow::Result<()>;
    fn remove_certified_aug_data(
        &self,
        certified_aug_data: Vec<CertifiedAugData<D>>,
    ) -> anyhow::Result<()>;
}
