// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::rand::rand_gen::{
    storage::interface::RandStorage,
    types::{
        AugData, AugDataId, AugDataSignature, CertifiedAugData, CertifiedAugDataAck, RandConfig,
        TAugmentedData,
    },
};
use anyhow::ensure;
use velor_consensus_types::common::Author;
use velor_logger::error;
use velor_types::validator_signer::ValidatorSigner;
use std::{collections::HashMap, sync::Arc};

pub struct AugDataStore<D> {
    epoch: u64,
    signer: Arc<ValidatorSigner>,
    config: RandConfig,
    fast_config: Option<RandConfig>,
    data: HashMap<Author, AugData<D>>,
    certified_data: HashMap<Author, CertifiedAugData<D>>,
    db: Arc<dyn RandStorage<D>>,
}

impl<D: TAugmentedData> AugDataStore<D> {
    fn filter_by_epoch<T>(
        epoch: u64,
        all_data: impl Iterator<Item = (AugDataId, T)>,
    ) -> (Vec<T>, Vec<(AugDataId, T)>) {
        let mut to_remove = vec![];
        let mut to_keep = vec![];
        for (id, data) in all_data {
            if id.epoch() != epoch {
                to_remove.push(data)
            } else {
                to_keep.push((id, data))
            }
        }
        (to_remove, to_keep)
    }

    pub fn new(
        epoch: u64,
        signer: Arc<ValidatorSigner>,
        config: RandConfig,
        fast_config: Option<RandConfig>,
        db: Arc<dyn RandStorage<D>>,
    ) -> Self {
        let all_data = db.get_all_aug_data().unwrap_or_default();
        let (to_remove, aug_data) = Self::filter_by_epoch(epoch, all_data.into_iter());
        if let Err(e) = db.remove_aug_data(to_remove) {
            error!("[AugDataStore] failed to remove aug data: {:?}", e);
        }

        let all_certified_data = db.get_all_certified_aug_data().unwrap_or_default();
        let (to_remove, certified_data) =
            Self::filter_by_epoch(epoch, all_certified_data.into_iter());
        if let Err(e) = db.remove_certified_aug_data(to_remove) {
            error!(
                "[AugDataStore] failed to remove certified aug data: {:?}",
                e
            );
        }

        for (_, certified_data) in &certified_data {
            certified_data
                .data()
                .augment(&config, &fast_config, certified_data.author());
        }

        Self {
            epoch,
            signer,
            config,
            fast_config,
            data: aug_data
                .into_iter()
                .map(|(id, data)| (id.author(), data))
                .collect(),
            certified_data: certified_data
                .into_iter()
                .map(|(id, data)| (id.author(), data))
                .collect(),
            db,
        }
    }

    pub fn get_my_aug_data(&self) -> Option<AugData<D>> {
        self.data.get(&self.config.author()).cloned()
    }

    pub fn get_my_certified_aug_data(&self) -> Option<CertifiedAugData<D>> {
        self.certified_data.get(&self.config.author()).cloned()
    }

    pub fn my_certified_aug_data_exists(&self) -> bool {
        self.certified_data.contains_key(&self.config.author())
    }

    pub fn add_aug_data(&mut self, data: AugData<D>) -> anyhow::Result<AugDataSignature> {
        if let Some(existing_data) = self.data.get(data.author()) {
            ensure!(
                existing_data == &data,
                "[AugDataStore] equivocate data from {}",
                data.author()
            );
        } else {
            self.db.save_aug_data(&data)?;
        }
        let sig = AugDataSignature::new(self.epoch, self.signer.sign(&data)?);
        self.data.insert(*data.author(), data);
        Ok(sig)
    }

    pub fn add_certified_aug_data(
        &mut self,
        certified_data: CertifiedAugData<D>,
    ) -> anyhow::Result<CertifiedAugDataAck> {
        if self.certified_data.contains_key(certified_data.author()) {
            return Ok(CertifiedAugDataAck::new(self.epoch));
        }
        self.db.save_certified_aug_data(&certified_data)?;
        certified_data
            .data()
            .augment(&self.config, &self.fast_config, certified_data.author());
        self.certified_data
            .insert(*certified_data.author(), certified_data);
        Ok(CertifiedAugDataAck::new(self.epoch))
    }
}
