// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    database::{
        clean_data_for_db, execute_with_better_error, get_chunks, PgDbPool, PgPoolConnection,
    },
    indexer::{
        errors::TransactionProcessingError, processing_result::ProcessingResult,
        transaction_processor::TransactionProcessor,
    },
    models::coin_models::{
        coin_activities::{CoinActivity, CurrentCoinBalancePK},
        coin_balances::{CoinBalance, CurrentCoinBalance},
        coin_infos::CoinInfo,
    },
    schema,
};
use aptos_api_types::Transaction as APITransaction;
use async_trait::async_trait;
use diesel::{pg::upsert::excluded, result::Error, ExpressionMethods, PgConnection};
use field_count::FieldCount;
use std::{collections::HashMap, fmt::Debug};

pub const NAME: &str = "coin_processor";
pub struct CoinTransactionProcessor {
    connection_pool: PgDbPool,
}

impl CoinTransactionProcessor {
    pub fn new(connection_pool: PgDbPool) -> Self {
        Self { connection_pool }
    }
}

impl Debug for CoinTransactionProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = &self.connection_pool.state();
        write!(
            f,
            "CoinTransactionProcessor {{ connections: {:?}  idle_connections: {:?} }}",
            state.connections, state.idle_connections
        )
    }
}

fn insert_to_db_impl(
    conn: &mut PgConnection,
    coin_activities: &[CoinActivity],
    coin_infos: &[CoinInfo],
    coin_balances: &[CoinBalance],
    current_coin_balances: &[CurrentCoinBalance],
) -> Result<(), diesel::result::Error> {
    insert_coin_activities(conn, coin_activities)?;
    insert_coin_infos(conn, coin_infos)?;
    insert_coin_balances(conn, coin_balances)?;
    insert_current_coin_balances(conn, current_coin_balances)?;
    Ok(())
}

fn insert_to_db(
    conn: &mut PgPoolConnection,
    name: &'static str,
    start_version: u64,
    end_version: u64,
    coin_activities: Vec<CoinActivity>,
    coin_infos: Vec<CoinInfo>,
    coin_balances: Vec<CoinBalance>,
    current_coin_balances: Vec<CurrentCoinBalance>,
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
                &coin_activities,
                &coin_infos,
                &coin_balances,
                &current_coin_balances,
            )
        }) {
        Ok(_) => Ok(()),
        Err(_) => conn
            .build_transaction()
            .read_write()
            .run::<_, Error, _>(|pg_conn| {
                let coin_activities = clean_data_for_db(coin_activities, true);
                let coin_infos = clean_data_for_db(coin_infos, true);
                let coin_balances = clean_data_for_db(coin_balances, true);
                let current_coin_balances = clean_data_for_db(current_coin_balances, true);

                insert_to_db_impl(
                    pg_conn,
                    &coin_activities,
                    &coin_infos,
                    &coin_balances,
                    &current_coin_balances,
                )
            }),
    }
}

fn insert_coin_activities(
    conn: &mut PgConnection,
    item_to_insert: &[CoinActivity],
) -> Result<(), diesel::result::Error> {
    use schema::coin_activities::dsl::*;

    let chunks = get_chunks(item_to_insert.len(), CoinActivity::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::coin_activities::table)
                .values(&item_to_insert[start_ind..end_ind])
                .on_conflict((
                    transaction_version,
                    event_account_address,
                    event_creation_number,
                    event_sequence_number,
                ))
                .do_update()
                .set((
                    owner_address.eq(excluded(owner_address)),
                    coin_type.eq(excluded(coin_type)),
                    amount.eq(excluded(amount)),
                    activity_type.eq(excluded(activity_type)),
                    is_gas_fee.eq(excluded(is_gas_fee)),
                    is_transaction_success.eq(excluded(is_transaction_success)),
                    entry_function_id_str.eq(excluded(entry_function_id_str)),
                    transaction_timestamp.eq(excluded(transaction_timestamp)),
                )),
            None,
        )?;
    }
    Ok(())
}

fn insert_coin_infos(
    conn: &mut PgConnection,
    item_to_insert: &[CoinInfo],
) -> Result<(), diesel::result::Error> {
    use schema::coin_infos::dsl::*;

    let chunks = get_chunks(item_to_insert.len(), CoinInfo::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::coin_infos::table)
                .values(&item_to_insert[start_ind..end_ind])
                .on_conflict(coin_type_hash)
                .do_update()
                .set((
                    transaction_version_created.eq(excluded(transaction_version_created)),
                    creator_address.eq(excluded(creator_address)),
                    name.eq(excluded(name)),
                    symbol.eq(excluded(symbol)),
                    decimals.eq(excluded(decimals)),
                    transaction_created_timestamp.eq(excluded(transaction_created_timestamp)),
                )),
            Some(" WHERE coin_infos.transaction_version_created >= EXCLUDED.transaction_version_created "),
        )?;
    }
    Ok(())
}

fn insert_coin_balances(
    conn: &mut PgConnection,
    item_to_insert: &[CoinBalance],
) -> Result<(), diesel::result::Error> {
    use schema::coin_balances::dsl::*;

    let chunks = get_chunks(item_to_insert.len(), CoinBalance::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::coin_balances::table)
                .values(&item_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, owner_address, coin_type_hash))
                .do_update()
                .set((
                    amount.eq(excluded(amount)),
                    transaction_timestamp.eq(excluded(transaction_timestamp)),
                )),
            None,
        )?;
    }
    Ok(())
}

fn insert_current_coin_balances(
    conn: &mut PgConnection,
    item_to_insert: &[CurrentCoinBalance],
) -> Result<(), diesel::result::Error> {
    use schema::current_coin_balances::dsl::*;

    let chunks = get_chunks(item_to_insert.len(), CurrentCoinBalance::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::current_coin_balances::table)
                .values(&item_to_insert[start_ind..end_ind])
                .on_conflict((owner_address, coin_type_hash))
                .do_update()
                .set((
                    amount.eq(excluded(amount)),
                    last_transaction_version.eq(excluded(last_transaction_version)),
                    last_transaction_timestamp.eq(excluded(last_transaction_timestamp)),
                )),
                Some(" WHERE current_coin_balances.last_transaction_version <= excluded.last_transaction_version "),
            )?;
    }
    Ok(())
}

#[async_trait]
impl TransactionProcessor for CoinTransactionProcessor {
    fn name(&self) -> &'static str {
        NAME
    }

    async fn process_transactions(
        &self,
        transactions: Vec<APITransaction>,
        start_version: u64,
        end_version: u64,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        let mut all_coin_activities = vec![];
        let mut all_coin_balances = vec![];
        let mut all_coin_infos: HashMap<String, CoinInfo> = HashMap::new();
        let mut all_current_coin_balances: HashMap<CurrentCoinBalancePK, CurrentCoinBalance> =
            HashMap::new();

        for txn in &transactions {
            let (mut coin_activities, mut coin_balances, coin_infos, current_coin_balances) =
                CoinActivity::from_transaction(txn);
            all_coin_activities.append(&mut coin_activities);
            all_coin_balances.append(&mut coin_balances);
            // For coin infos, we only want to keep the first version, so insert only if key is not present already
            for (key, value) in coin_infos {
                all_coin_infos.entry(key).or_insert(value);
            }
            all_current_coin_balances.extend(current_coin_balances);
        }
        let mut all_coin_infos = all_coin_infos.into_values().collect::<Vec<CoinInfo>>();
        let mut all_current_coin_balances = all_current_coin_balances
            .into_values()
            .collect::<Vec<CurrentCoinBalance>>();

        // Sort by PK
        all_coin_infos.sort_by(|a, b| a.coin_type.cmp(&b.coin_type));
        all_current_coin_balances.sort_by(|a, b| {
            (&a.owner_address, &a.coin_type).cmp(&(&b.owner_address, &b.coin_type))
        });

        let mut conn = self.get_conn();
        let tx_result = insert_to_db(
            &mut conn,
            self.name(),
            start_version,
            end_version,
            all_coin_activities,
            all_coin_infos,
            all_coin_balances,
            all_current_coin_balances,
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
