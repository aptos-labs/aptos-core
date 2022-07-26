// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub(crate) mod metadata;

use super::AptosDB;
use crate::indexer::metadata::{Metadata, MetadataTag};
use crate::indexer_metadata::IndexerMetadataSchema;
use crate::{TableInfoSchema, OTHER_TIMERS_SECONDS};
///! This temporarily implements node internal indexing functionalities on the AptosDB.
use anyhow::{bail, ensure, Error, Result};
use aptos_types::access_path::Path;
use aptos_types::account_address::AccountAddress;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::state_store::table::TableHandle;
use aptos_types::state_store::table::TableInfo;
use aptos_types::transaction::{TransactionToCommit, Version};
use aptos_types::write_set::{WriteOp, WriteSet};
use aptos_vm::data_cache::{AsMoveResolver, RemoteStorage};
use move_deps::move_core_types::identifier::IdentStr;
use move_deps::move_core_types::language_storage::{StructTag, TypeTag};
use move_deps::move_resource_viewer::{AnnotatedMoveValue, MoveValueAnnotator};
use schemadb::SchemaBatch;
use std::collections::HashMap;
use std::convert::TryInto;
use storage_interface::state_view::DbStateView;
use storage_interface::DbReader;

impl AptosDB {
    pub fn index_transactions(
        &self,
        last_version: Version,
        txns_to_commit: &[TransactionToCommit],
    ) -> Result<()> {
        if txns_to_commit.is_empty() {
            return Ok(());
        }
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["index_transactions"])
            .start_timer();

        let write_sets: Vec<_> = txns_to_commit.iter().map(|t| t.write_set()).collect();

        self.index_write_sets(last_version, &write_sets)
    }

    fn index_write_sets(&self, latest_version: Version, write_sets: &[&WriteSet]) -> Result<()> {
        let state_store = self.state_store.clone();
        let state_view = DbStateView {
            db: state_store,
            version: Some(latest_version),
        };
        let resolver = state_view.as_move_resolver();
        let annotator = MoveValueAnnotator::new(&resolver);
        let mut table_info_parser = TableInfoParser::new(self, &annotator);

        for write_set in write_sets {
            for (state_key, write_op) in write_set.iter() {
                table_info_parser.parse_write_op(state_key, write_op)?;
            }
        }

        let mut batch = SchemaBatch::new();
        table_info_parser.finish(&mut batch)?;
        batch.put::<IndexerMetadataSchema>(
            &MetadataTag::LatestVersion,
            &Metadata::LatestVersion(latest_version),
        )?;
        self.index_db.write_schemas(batch)?;

        Ok(())
    }

    pub fn catchup_indexer(&self) -> Result<()> {
        let ledger_version = self
            .get_latest_transaction_info_option()?
            .map(|(v, _)| v + 1);
        if let Some(ledger_version) = ledger_version {
            let indexer_next_version = self.get_indexer_version()?.map_or(0, |v| v + 1);
            if indexer_next_version <= ledger_version {
                let write_sets = self
                    .transaction_store
                    .get_write_sets(indexer_next_version, ledger_version + 1)?;
                let write_sets_ref: Vec<_> = write_sets.iter().collect();
                self.index_write_sets(ledger_version, &write_sets_ref)?;
            }
        }
        Ok(())
    }

    pub fn get_indexer_version(&self) -> Result<Option<Version>> {
        Ok(self
            .index_db
            .get::<IndexerMetadataSchema>(&MetadataTag::LatestVersion)?
            .map(|data| match data {
                Metadata::LatestVersion(version) => version,
            }))
    }
}

struct TableInfoParser<'a> {
    db: &'a AptosDB,
    annotator: &'a MoveValueAnnotator<'a, RemoteStorage<'a, DbStateView>>,
    result: HashMap<TableHandle, TableInfo>,
    pending_on: HashMap<TableHandle, Vec<&'a [u8]>>,
}

impl<'a> TableInfoParser<'a> {
    pub fn new(
        db: &'a AptosDB,
        annotator: &'a MoveValueAnnotator<RemoteStorage<DbStateView>>,
    ) -> Self {
        Self {
            db,
            annotator,
            result: HashMap::new(),
            pending_on: HashMap::new(),
        }
    }

    pub fn parse_write_op(&mut self, state_key: &'a StateKey, write_op: &'a WriteOp) -> Result<()> {
        match write_op {
            WriteOp::Value(bytes) => match state_key {
                StateKey::AccessPath(access_path) => {
                    let path: Path = (&access_path.path).try_into()?;
                    match path {
                        Path::Code(_) => (),
                        Path::Resource(struct_tag) => self.parse_struct(struct_tag, bytes)?,
                    }
                }
                StateKey::TableItem { handle, .. } => self.parse_table_item(*handle, bytes)?,
                StateKey::Raw(_) => (),
            },
            WriteOp::Deletion => (),
            WriteOp::Delta(_, _) => (),
        }
        Ok(())
    }

    fn parse_struct(&mut self, struct_tag: StructTag, bytes: &[u8]) -> Result<()> {
        self.parse_move_value(
            &self
                .annotator
                .view_value(&TypeTag::Struct(struct_tag), bytes)?,
        )
    }

    fn parse_table_item(&mut self, handle: TableHandle, bytes: &'a [u8]) -> Result<()> {
        match self.get_table_info(handle)? {
            Some(table_info) => {
                self.parse_move_value(&self.annotator.view_value(&table_info.value_type, bytes)?)?;
            }
            None => {
                self.pending_on
                    .entry(handle)
                    .or_insert_with(Vec::new)
                    .push(bytes);
            }
        }
        Ok(())
    }

    fn parse_move_value(&mut self, move_value: &AnnotatedMoveValue) -> Result<()> {
        match move_value {
            AnnotatedMoveValue::Vector(_type_tag, items) => {
                for item in items {
                    self.parse_move_value(item)?;
                }
            }
            AnnotatedMoveValue::Struct(struct_value) => {
                let struct_tag = &struct_value.type_;
                if Self::is_table(struct_tag) {
                    assert_eq!(struct_tag.type_params.len(), 2);
                    let table_info = TableInfo {
                        key_type: struct_tag.type_params[0].clone(),
                        value_type: struct_tag.type_params[1].clone(),
                    };
                    let table_handle = match &struct_value.value[0] {
                        (name, AnnotatedMoveValue::U128(handle)) => {
                            assert_eq!(name.as_ref(), IdentStr::new("handle").unwrap());
                            TableHandle(*handle)
                        }
                        _ => bail!("Table struct malformed. {:?}", struct_value),
                    };
                    self.save_table_info(table_handle, table_info)?;
                } else {
                    for (_identifier, field) in &struct_value.value {
                        self.parse_move_value(field)?;
                    }
                }
            }

            // there won't be tables in primitives
            AnnotatedMoveValue::U8(_) => {}
            AnnotatedMoveValue::U64(_) => {}
            AnnotatedMoveValue::U128(_) => {}
            AnnotatedMoveValue::Bool(_) => {}
            AnnotatedMoveValue::Address(_) => {}
            AnnotatedMoveValue::Bytes(_) => {}
        }
        Ok(())
    }

    fn save_table_info(&mut self, handle: TableHandle, info: TableInfo) -> Result<()> {
        if self.get_table_info(handle)?.is_none() {
            self.result.insert(handle, info);
            if let Some(pending_items) = self.pending_on.remove(&handle) {
                for bytes in pending_items {
                    self.parse_table_item(handle, bytes)?;
                }
            }
        }
        Ok(())
    }

    fn is_table(struct_tag: &StructTag) -> bool {
        struct_tag.address == AccountAddress::ONE
            && struct_tag.module.as_ref() == IdentStr::new("table").unwrap()
            && struct_tag.name.as_ref() == IdentStr::new("Table").unwrap()
    }

    fn get_table_info(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        match self.result.get(&handle) {
            Some(table_info) => Ok(Some(table_info.clone())),
            None => self.db.get_table_info_option(handle),
        }
    }

    fn finish(self, batch: &mut SchemaBatch) -> Result<bool> {
        ensure!(
            self.pending_on.is_empty(),
            "There is still pending table items to parse due to unknown table info for table handles: {:?}",
            self.pending_on.keys(),
        );

        if self.result.is_empty() {
            Ok(false)
        } else {
            self.result
                .into_iter()
                .try_for_each(|(table_handle, table_info)| {
                    batch.put::<TableInfoSchema>(&table_handle, &table_info)
                })?;
            Ok(true)
        }
    }
}
