// Copyright © Aptos Foundation

use crate::{
    database::{
        clean_data_for_db, execute_with_better_error, get_chunks, PgDbPool, PgPoolConnection,
    },
    indexer::{
        errors::TransactionProcessingError, processing_result::ProcessingResult,
        transaction_processor::TransactionProcessor,
    },
    models::vault_models::{
        vault_resources::{UserInfo, Vault},
        vault_activities::VaultActivity,
    },
    schema,
};
use aptos_api_types::Transaction as APITransaction;
use async_trait::async_trait;
use diesel::{pg::upsert::excluded, result::Error, ExpressionMethods, PgConnection};
use field_count::FieldCount;
use std::{collections::HashMap, fmt::Debug};
use aptos_logger::info;

pub const NAME: &str = "vault_processor";
pub struct VaultProcessor {
    connection_pool: PgDbPool,
}

impl VaultProcessor {
    pub fn new(connection_pool: PgDbPool) -> Self {
        Self { connection_pool }
    }
}

impl Debug for VaultProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = &self.connection_pool.state();
        write!(
            f,
            "VaultProcessor {{ connections: {:?}  idle_connections: {:?} }}",
            state.connections, state.idle_connections
        )
    }
}

fn insert_to_db_impl(
    conn: &mut PgConnection,
    all_user_infos: &[UserInfo],
    all_vaults: &[Vault],
    all_vault_activities: &[VaultActivity],
) -> Result<(), diesel::result::Error> {
    insert_user_info(conn, all_user_infos)?;
    insert_vaults(conn, all_vaults)?;
    insert_vault_activities(conn, all_vault_activities)?;
    Ok(())
}

fn insert_to_db(
    conn: &mut PgPoolConnection,
    name: &'static str,
    start_version: u64,
    end_version: u64,
    all_user_infos: Vec<UserInfo>,
    all_vaults: Vec<Vault>,
    all_vault_activities: Vec<VaultActivity>,
) -> Result<(), diesel::result::Error> {
    aptos_logger::trace!(
        name = name,
        start_version = start_version,
        end_version = end_version,
        "Inserting to db",
    );
    match conn
        .build_transaction()
        .read_write()
        .run::<_, Error, _>(|pg_conn| {
            insert_to_db_impl(
                pg_conn,
                &all_user_infos,
                &all_vaults,
                &all_vault_activities,
            )
        }) {
        Ok(_) => Ok(()),
        Err(_) => conn
            .build_transaction()
            .read_write()
            .run::<_, Error, _>(|pg_conn| {
                let all_user_infos = clean_data_for_db(all_user_infos, true);
                let all_vaults = clean_data_for_db(all_vaults, true);

                insert_to_db_impl(
                    pg_conn,
                    &all_user_infos,
                    &all_vaults,
                    &all_vault_activities,
                )
            }),
    }
}

fn insert_user_info(
    conn: &mut PgConnection,
    item_to_insert: &[UserInfo],
) -> Result<(), diesel::result::Error> {
    use schema::user_infos::dsl::*;

    let chunks = get_chunks(item_to_insert.len(), UserInfo::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::user_infos::table)
                .values(&item_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, user_address, type_hash))
                .do_update()
                .set((
                    transaction_version.eq(excluded(transaction_version)),
                    transaction_timestamp.eq(excluded(transaction_timestamp)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
                None,
            )?;
    }
    Ok(())
}

fn insert_vaults(
    conn: &mut PgConnection,
    item_to_insert: &[Vault],
) -> Result<(), diesel::result::Error> {
    use schema::vaults::dsl::*;

    let chunks = get_chunks(item_to_insert.len(), Vault::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::vaults::table)
                .values(&item_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, type_hash))
                .do_update()
                .set((
                    transaction_version.eq(excluded(transaction_version)),
                    transaction_timestamp.eq(excluded(transaction_timestamp)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
                None,
            )?;
    }
    Ok(())
}

fn insert_vault_activities(
    conn: &mut PgConnection,
    item_to_insert: &[VaultActivity],
) -> Result<(), diesel::result::Error> {
    use schema::vault_activities::dsl::*;

    let chunks = get_chunks(item_to_insert.len(), VaultActivity::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::vault_activities::table)
                .values(&item_to_insert[start_ind..end_ind])
                .on_conflict((
                    transaction_version,
                    event_creation_number,
                    event_sequence_number,
                ))
                .do_update()
                .set((
                    inserted_at.eq(excluded(inserted_at)),
                    event_index.eq(excluded(event_index)),
                )),
            None,
        )?;
    }
    Ok(())
}

#[async_trait]
impl TransactionProcessor for VaultProcessor {
    fn name(&self) -> &'static str {
        NAME
    }

    async fn process_transactions(
        &self,
        transactions: Vec<APITransaction>,
        start_version: u64,
        end_version: u64,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        let mut conn = self.get_conn();

        let mut all_vault_activities: Vec<VaultActivity> = vec![];
        let mut all_user_infos: HashMap<(String, String), UserInfo> = HashMap::new();
        let mut all_vaults: HashMap<(String, String), Vault> = HashMap::new();

        info!(
            "VaultProcessor {{ processing: {:?} start version: {:?} end_version: {:?}}}",
            transactions.len(), start_version, end_version
        );

        for txn in &transactions {
            let (mut vault_activities, user_infos, vaults) = VaultActivity::from_transaction(
                txn,
            );
            all_vault_activities.append(&mut vault_activities);
            all_user_infos.extend(user_infos);
            all_vaults.extend(vaults);
        }

        let mut all_user_infos = all_user_infos.into_values().collect::<Vec<UserInfo>>();
        let mut all_vaults = all_vaults.into_values().collect::<Vec<Vault>>();

        // Sort by user address, vault type
        all_user_infos.sort_by(|a, b| (&a.user_address, &a.collateral_type, &a.borrow_type)
            .cmp(&(&b.user_address, &b.collateral_type, &b.borrow_type)));

        // Sort by vault type
        all_vaults.sort_by(|a, b| (&a.collateral_type, &a.borrow_type)
            .cmp(&(&b.collateral_type, &b.borrow_type)));

        let tx_result = insert_to_db(
            &mut conn,
            self.name(),
            start_version,
            end_version,
            all_user_infos,
            all_vaults,
            all_vault_activities,
        );
        match tx_result {
            Ok(_) => Ok(ProcessingResult::new(
                self.name(),
                start_version,
                end_version,
            )),
            Err(err) => Err(TransactionProcessingError::TransactionCommitError((
                anyhow::Error::from(err),
                start_version,
                end_version,
                self.name(),
            ))),
        }
    }

    fn connection_pool(&self) -> &PgDbPool {
        &self.connection_pool
    }
}
