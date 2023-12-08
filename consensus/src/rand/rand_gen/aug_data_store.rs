// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::rand::rand_gen::{
    storage::interface::AugDataStorage,
    types::{AugData, AugDataId, CertifiedAugData, RandConfig},
};
use aptos_consensus_types::common::Author;
use aptos_logger::error;
use std::{collections::HashMap, sync::Arc};

pub struct AugDataStore<D, Storage> {
    config: RandConfig,
    data: HashMap<Author, AugData<D>>,
    certified_data: HashMap<Author, CertifiedAugData<D>>,
    db: Arc<Storage>,
}

impl<D, Storage: AugDataStorage<D>> AugDataStore<D, Storage> {
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

    pub fn new(epoch: u64, db: Arc<Storage>, config: RandConfig) -> Self {
        let all_data = db.get_all_aug_data().unwrap_or_default();
        let (to_remove, aug_data) = Self::filter_by_epoch(epoch, all_data.into_iter());
        if let Err(e) = db.remove_aug_data(to_remove.into_iter()) {
            error!("[AugDataStore] failed to remove aug data: {:?}", e);
        }

        let all_certified_data = db.get_all_certified_aug_data().unwrap_or_default();
        let (to_remove, certified_data) =
            Self::filter_by_epoch(epoch, all_certified_data.into_iter());
        if let Err(e) = db.remove_certified_aug_data(to_remove.into_iter()) {
            error!(
                "[AugDataStore] failed to remove certified aug data: {:?}",
                e
            );
        }

        Self {
            config,
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
}
