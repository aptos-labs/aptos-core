// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::rand::rand_gen::{
    storage::interface::AugDataStorage,
    types::{AugData, AugDataId, AugmentedData, CertifiedAugData},
};
use aptos_infallible::RwLock;
use std::collections::HashMap;

pub struct InMemRandDb<D> {
    aug_data: RwLock<HashMap<AugDataId, AugData<D>>>,
    certified_aug_data: RwLock<HashMap<AugDataId, CertifiedAugData<D>>>,
}

impl<D> InMemRandDb<D> {
    pub fn new() -> Self {
        Self {
            aug_data: RwLock::new(HashMap::new()),
            certified_aug_data: RwLock::new(HashMap::new()),
        }
    }
}

impl<D: AugmentedData> AugDataStorage<D> for InMemRandDb<D> {
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
