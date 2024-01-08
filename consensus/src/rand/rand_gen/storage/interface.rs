// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::rand::rand_gen::types::{AugData, AugDataId, CertifiedAugData};

pub trait AugDataStorage<D>: 'static {
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
