// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::AptosDB;
use crate::{TableInfoSchema, OTHER_TIMERS_SECONDS};
///! This temporarily implements node internal indexing functionalities on the AptosDB.
use anyhow::{bail, Result};
use aptos_types::access_path::Path;
use aptos_types::account_address::AccountAddress;
use aptos_types::state_store::state_key::StateKey;
use aptos_types::state_store::table::TableHandle;
use aptos_types::state_store::table::TableInfo;
use aptos_types::transaction::TransactionToCommit;
use aptos_types::write_set::WriteOp;
use aptos_vm::data_cache::AsMoveResolver;
use move_deps::move_core_types::identifier::IdentStr;
use move_deps::move_core_types::language_storage::{StructTag, TypeTag};
use move_deps::move_core_types::resolver::MoveResolver;
use move_deps::move_resource_viewer::{AnnotatedMoveValue, MoveValueAnnotator};
use schemadb::SchemaBatch;
use std::convert::TryInto;
use storage_interface::state_view::DbStateView;
use storage_interface::DbReader;

impl AptosDB {
    pub fn index_transactions(&self, txns_to_commit: &[TransactionToCommit]) -> Result<()> {
        if txns_to_commit.is_empty() {
            return Ok(());
        }
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["index_transactions"])
            .start_timer();

        let state_store = self.state_store.clone();
        let state_view = DbStateView {
            db: state_store,
            version: self
                .ledger_store
                .get_latest_transaction_info_option()?
                .map(|(v, _)| v + 1),
        };
        let resolver = state_view.as_move_resolver();
        let annotator = MoveValueAnnotator::new(&resolver);

        let mut batch = SchemaBatch::new();
        for txn_to_commit in txns_to_commit {
            for (state_key, write_op) in txn_to_commit.write_set() {
                self.parse_table_info_from_write_op(&annotator, state_key, write_op, &mut batch)?;
            }
        }
        self.index_db.write_schemas(batch)
    }

    fn parse_table_info_from_write_op(
        &self,
        annotator: &MoveValueAnnotator<impl MoveResolver>,
        state_key: &StateKey,
        write_op: &WriteOp,
        batch: &mut SchemaBatch,
    ) -> Result<()> {
        match write_op {
            WriteOp::Value(bytes) => match state_key {
                StateKey::AccessPath(access_path) => {
                    let path: Path = (&access_path.path).try_into()?;
                    match path {
                        Path::Code(_) => (),
                        Path::Resource(struct_tag) => self.parse_table_info(
                            &annotator.view_value(&TypeTag::Struct(struct_tag), bytes)?,
                            batch,
                        )?,
                    }
                }
                StateKey::TableItem { handle, .. } => {
                    let table_info = self.get_table_info(*handle)?;
                    self.parse_table_info(
                        &annotator.view_value(&table_info.value_type, bytes)?,
                        batch,
                    )?
                }
                StateKey::Raw(_) => (),
            },
            WriteOp::Deletion => (),
            WriteOp::Delta(_, _) => (),
        }
        Ok(())
    }

    fn parse_table_info(
        &self,
        move_value: &AnnotatedMoveValue,
        batch: &mut SchemaBatch,
    ) -> Result<()> {
        match move_value {
            AnnotatedMoveValue::Vector(_type_tag, items) => {
                for item in items {
                    self.parse_table_info(item, batch)?;
                }
            }
            AnnotatedMoveValue::Struct(struct_value) => {
                let struct_tag = &struct_value.type_;
                if Self::is_table(&struct_tag) {
                    assert_eq!(struct_tag.type_params.len(), 2);
                    let table_info = TableInfo {
                        key_type: struct_tag.type_params[0].clone(),
                        value_type: struct_tag.type_params[1].clone(),
                    };
                    let table_handle = match &struct_value.value[0] {
                        (name, AnnotatedMoveValue::U128(handle)) => {
                            assert_eq!(name.as_ref(), IdentStr::new("handle").unwrap());
                            println!("found table. {} {:?}", handle, table_info);
                            TableHandle(*handle)
                        }
                        _ => bail!("Table struct malformed. {:?}", struct_value),
                    };
                    batch.put::<TableInfoSchema>(&table_handle, &table_info)?;
                } else {
                    for (_identifier, field) in &struct_value.value {
                        self.parse_table_info(&field, batch)?;
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

    fn is_table(struct_tag: &StructTag) -> bool {
        struct_tag.address == AccountAddress::ONE
            && struct_tag.module.as_ref() == IdentStr::new("table").unwrap()
            && struct_tag.name.as_ref() == IdentStr::new("Table").unwrap()
    }
}
