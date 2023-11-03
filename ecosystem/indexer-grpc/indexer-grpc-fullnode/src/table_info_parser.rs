// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, ensure, Result};

use aptos_logger::{info, error};
use aptos_sdk::bcs;
use aptos_types::{
    access_path::Path,
    account_address::AccountAddress,
    state_store::{
        state_key::{StateKey, StateKeyInner},
        table::{TableHandle, TableInfo},
    },
    transaction::Version,
    write_set::{WriteOp, WriteSet},
};

use bytes::Bytes;
use deadpool_postgres::{Pool, Object};
use futures::Future;
use move_core_types::{
    ident_str,
    language_storage::{StructTag, TypeTag},
    resolver::MoveResolver,
};
use move_resource_viewer::{AnnotatedMoveValue, MoveValueAnnotator};
use tokio_postgres::types::ToSql;
use std::{
    collections::{BTreeMap, HashMap},
    convert::TryInto, sync::Arc, str::FromStr, pin::Pin,
};

#[derive(Clone)]
pub struct IndexerLookupDB {
    pub pool: Arc<Pool>,
}

impl IndexerLookupDB {
    pub async fn index_with_annotator<R: MoveResolver>(
        &self,
        annotator: &MoveValueAnnotator<'_, R>,
        first_version: Version,
        write_sets: &[&WriteSet],
    ) -> Result<()> {
        let end_version = first_version + write_sets.len() as Version;
        let mut table_info_parser = TableInfoParser::new(self, annotator);
        for write_set in write_sets {
            for (state_key, write_op) in write_set.iter() {
                table_info_parser.parse_write_op(state_key, write_op).await?;
            }
        }
        let client = self.pool.clone().get().await.unwrap();

        match table_info_parser.finish(client).await {
            Ok(_) => {},
            // do i need to change this to handle it with multi thread?
            Err(err) => {
                aptos_logger::error!(first_version = first_version, end_version = end_version, error = ?&err);
                write_sets
                .iter()
                .enumerate()
                .for_each(|(i, write_set)| {
                    aptos_logger::error!(version = first_version as usize + i, write_set = ?write_set);
                });
                bail!(err);
            },
        };
        Ok(())
    }

    pub async fn get_table_info(&self, handle: TableHandle) -> Result<Option<TableInfo>> {

        let query = "SELECT * FROM table_metadatas WHERE handle = $1";
        let client = self.pool.get().await.unwrap();
        let table_info_query = client.prepare(query).await.unwrap();
    
        match client.query(&table_info_query, &[&handle.0.to_standard_string()]).await {
            Ok(rows) => {
                if let Some(row) = rows.iter().next() {
                    let key_type_str: &str = row.get(0);
                    let value_type_str: &str = row.get(1);
    
                    let key_type = TypeTag::from_str(key_type_str).unwrap();
                    let value_type = TypeTag::from_str(value_type_str).unwrap();
    
                    Ok(Some(TableInfo {
                        key_type,
                        value_type,
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(err) => {
                error!("Error querying table_metadatas: {}", err);
                Err(err.into())
            }
        }
    }
}

struct TableInfoParser<'a, R> {
    indexer: &'a IndexerLookupDB,
    annotator: &'a MoveValueAnnotator<'a, R>,
    result: HashMap<TableHandle, TableInfo>,
    pending_on: HashMap<TableHandle, Vec<Bytes>>,
}

impl<'a, R: MoveResolver> TableInfoParser<'a, R> {
    pub fn new(indexer: &'a IndexerLookupDB, annotator: &'a MoveValueAnnotator<R>) -> Self {
        Self {
            indexer,
            annotator,
            result: HashMap::new(),
            pending_on: HashMap::new(),
        }
    }

    pub async fn parse_write_op(&mut self, state_key: &'a StateKey, write_op: &'a WriteOp) -> Result<()> {
        if let Some(bytes) = write_op.bytes() {
            match state_key.inner() {
                StateKeyInner::AccessPath(access_path) => {
                    let path: Path = (&access_path.path).try_into()?;
                    match path {
                        Path::Code(_) => (),
                        Path::Resource(struct_tag) => self.parse_struct(struct_tag, bytes).await?,
                        Path::ResourceGroup(_struct_tag) => self.parse_resource_group(bytes).await?,
                    }
                },
                StateKeyInner::TableItem { handle, .. } => self.parse_table_item(*handle, bytes).await?,
                StateKeyInner::Raw(_) => (),
            }
        }
        Ok(())
    }

    async fn parse_struct(&mut self, struct_tag: StructTag, bytes: &Bytes) -> Result<()> {
        self.parse_move_value(
            &self
                .annotator
                .view_value(&TypeTag::Struct(Box::new(struct_tag)), bytes)?,
        ).await
    }

    async fn parse_resource_group(&mut self, bytes: &Bytes) -> Result<()> {
        type ResourceGroup = BTreeMap<StructTag, Bytes>;

        for (struct_tag, bytes) in bcs::from_bytes::<ResourceGroup>(bytes)? {
            self.parse_struct(struct_tag, &bytes).await?;
        }
        Ok(())
    }

    async fn parse_table_item(&mut self, handle: TableHandle, bytes: &Bytes) -> Result<()> {
        match self.get_table_info(handle).await? {
            Some(table_info) => {
                self.parse_move_value(&self.annotator.view_value(&table_info.value_type, bytes)?).await?;
            },
            None => {
                self.pending_on
                    .entry(handle)
                    .or_insert_with(Vec::new)
                    .push(bytes.clone());
            },
        }
        Ok(())
    }

    fn parse_move_value(&mut self, move_value: &AnnotatedMoveValue) -> Pin<Box<dyn '_ + Future<Output = Result<()>>>> {
        Box::pin(async move {
            match move_value {
                AnnotatedMoveValue::Vector(_type_tag, items) => {
                    for item in items {
                        self.parse_move_value(item).await?;
                    }
                },
                AnnotatedMoveValue::Struct(struct_value) => {
                    let struct_tag = &struct_value.type_;
                    if Self::is_table(struct_tag) {
                        assert_eq!(struct_tag.type_params.len(), 2);
                        let table_info = TableInfo {
                            key_type: struct_tag.type_params[0].clone(),
                            value_type: struct_tag.type_params[1].clone(),
                        };
                        let table_handle = match &struct_value.value[0] {
                            (name, AnnotatedMoveValue::Address(handle)) => {
                                assert_eq!(name.as_ref(), ident_str!("handle"));
                                TableHandle(*handle)
                            },
                            _ => bail!("Table struct malformed. {:?}", struct_value),
                        };
                        self.save_table_info(table_handle, table_info).await?;
                    } else {
                        for (_identifier, field) in &struct_value.value {
                            self.parse_move_value(field).await?;
                        }
                    }
                },

                // there won't be tables in primitives
                AnnotatedMoveValue::U8(_) => {},
                AnnotatedMoveValue::U16(_) => {},
                AnnotatedMoveValue::U32(_) => {},
                AnnotatedMoveValue::U64(_) => {},
                AnnotatedMoveValue::U128(_) => {},
                AnnotatedMoveValue::U256(_) => {},
                AnnotatedMoveValue::Bool(_) => {},
                AnnotatedMoveValue::Address(_) => {},
                AnnotatedMoveValue::Bytes(_) => {},
            }
            Ok(())
        })
    }

    async fn save_table_info(&mut self, handle: TableHandle, info: TableInfo) -> Result<()> {
        if self.get_table_info(handle).await?.is_none() {
            self.result.insert(handle, info);
            if let Some(pending_items) = self.pending_on.remove(&handle) {
                for bytes in pending_items {
                    self.parse_table_item(handle, &bytes).await?;
                }
            }
        }
        Ok(())
    }

    fn is_table(struct_tag: &StructTag) -> bool {
        struct_tag.address == AccountAddress::ONE
            && struct_tag.module.as_ident_str() == ident_str!("table")
            && struct_tag.name.as_ident_str() == ident_str!("Table")
    }

    async fn get_table_info(&self, handle: TableHandle) -> Result<Option<TableInfo>> {
        match self.result.get(&handle) {
            Some(table_info) => Ok(Some(table_info.clone())),
            None => self.indexer.get_table_info(handle).await,
        }
    }

    async fn finish(self, client: Object) -> Result<bool> {
        ensure!(
            self.pending_on.is_empty(),
            "There are still pending table items to parse due to unknown table info for table handles: {:?}",
            self.pending_on.keys(),
        );

        let table_name = "";
        let column_name: Vec<&str> = vec!["handle", "key_type", "value_type"];
        let table_values = "";
        let pk_str = "handle";

        if self.result.is_empty() {
            return Ok(false);
        }
        for (table_handle, table_info) in self.result {
            let table_handle_standard = table_handle.0.to_standard_string();
            let key_type_canonical = table_info.key_type.to_canonical_string();
            let value_type_canonical = table_info.value_type.to_canonical_string();
            let query_params: Vec<&(dyn ToSql + Sync)> = vec![
                &table_handle_standard,
                &key_type_canonical,
                &value_type_canonical,
            ];
            let query = format!(
                "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO NOTHING",
                table_name,
                column_name.join(", "),
                table_values,
                pk_str
            );

            let result = client.execute(query.as_str(), &query_params).await;
            match result {
                Ok(row) => {
                    info!("Write Operation was successful.");
                    info!("Number of rows written: {}", row);
                }
                Err(error) => {
                    error!("Error occurred in write: {}", error);
                }
            }
        }

        Ok(true)
    }
}
