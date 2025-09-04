// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_db_indexer_schemas::schema::state_keys::StateKeysSchema;
use velor_schemadb::{iterator::SchemaIterator, ReadOptions, DB};
use velor_storage_interface::{DbReader, Result};
use velor_types::{
    state_store::{
        state_key::{prefix::StateKeyPrefix, StateKey},
        state_value::StateValue,
    },
    transaction::Version,
};
use std::sync::Arc;

pub struct PrefixedStateValueIterator<'a> {
    state_keys_iter: SchemaIterator<'a, StateKeysSchema>,
    main_db: Arc<dyn DbReader>,
    key_prefix: StateKeyPrefix,
    desired_version: Version, // state values before the version
    is_finished: bool,
}

impl<'a> PrefixedStateValueIterator<'a> {
    pub fn new(
        main_db_reader: Arc<dyn DbReader>,
        indexer_db: &'a DB,
        key_prefix: StateKeyPrefix,
        first_key: Option<StateKey>,
        desired_version: Version,
    ) -> Result<Self> {
        let mut read_opt = ReadOptions::default();
        read_opt.set_total_order_seek(true);
        let mut state_keys_iter = indexer_db.iter_with_opts::<StateKeysSchema>(read_opt)?;
        if let Some(first_key) = first_key {
            state_keys_iter.seek(&first_key)?;
        } else {
            state_keys_iter.seek(&&key_prefix)?;
        };
        Ok(Self {
            state_keys_iter,
            main_db: main_db_reader,
            key_prefix,
            desired_version,
            is_finished: false,
        })
    }

    pub fn next_impl(&mut self) -> anyhow::Result<Option<(StateKey, StateValue)>> {
        let iter = &mut self.state_keys_iter;
        if self.is_finished {
            return Ok(None);
        }
        while let Some((state_key, _)) = iter.next().transpose()? {
            if !self.key_prefix.is_prefix(&state_key)? {
                self.is_finished = true;
                return Ok(None);
            }

            match self
                .main_db
                .get_state_value_by_version(&state_key, self.desired_version)?
            {
                Some(state_value) => {
                    return Ok(Some((state_key, state_value)));
                },
                None => {
                    // state key doesn't have value before the desired version, continue to next state key
                    continue;
                },
            }
        }
        Ok(None)
    }
}

impl Iterator for PrefixedStateValueIterator<'_> {
    type Item = anyhow::Result<(StateKey, StateValue)>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_impl().transpose()
    }
}
