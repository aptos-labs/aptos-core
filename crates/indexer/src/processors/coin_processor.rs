// Copyright © Velor Foundation
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
        account_transactions::AccountTransaction,
        coin_activities::{CoinActivity, CurrentCoinBalancePK},
        coin_balances::{CoinBalance, CurrentCoinBalance},
        coin_infos::{CoinInfo, CoinInfoQuery},
        coin_supply::CoinSupply,
    },
    schema,
};
use velor_api_types::Transaction as APITransaction;
use velor_types::{VelorCoinType, CoinType};
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
    coin_supply: &[CoinSupply],
    account_transactions: &[AccountTransaction],
) -> Result<(), diesel::result::Error> {
    insert_coin_activities(conn, coin_activities)?;
    insert_coin_infos(conn, coin_infos)?;
    insert_coin_balances(conn, coin_balances)?;
    insert_current_coin_balances(conn, current_coin_balances)?;
    insert_coin_supply(conn, coin_supply)?;
    insert_account_transactions(conn, account_transactions)?;
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
    coin_supply: Vec<CoinSupply>,
    account_transactions: Vec<AccountTransaction>,
) -> Result<(), diesel::result::Error> {
    velor_logger::trace!(
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
                &coin_supply,
                &account_transactions,
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
                let coin_supply = clean_data_for_db(coin_supply, true);
                let account_transactions = clean_data_for_db(account_transactions, true);

                insert_to_db_impl(
                    pg_conn,
                    &coin_activities,
                    &coin_infos,
                    &coin_balances,
                    &current_coin_balances,
                    &coin_supply,
                    &account_transactions,
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
                .do_nothing(),
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
                    supply_aggregator_table_handle.eq(excluded(supply_aggregator_table_handle)),
                    supply_aggregator_table_key.eq(excluded(supply_aggregator_table_key)),
                    inserted_at.eq(excluded(inserted_at)),
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
                .do_nothing(),
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
                    inserted_at.eq(excluded(inserted_at)),
                )),
                Some(" WHERE current_coin_balances.last_transaction_version <= excluded.last_transaction_version "),
            )?;
    }
    Ok(())
}

fn insert_coin_supply(
    conn: &mut PgConnection,
    item_to_insert: &[CoinSupply],
) -> Result<(), diesel::result::Error> {
    use schema::coin_supply::dsl::*;

    let chunks = get_chunks(item_to_insert.len(), CoinSupply::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::coin_supply::table)
                .values(&item_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, coin_type_hash))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_account_transactions(
    conn: &mut PgConnection,
    item_to_insert: &[AccountTransaction],
) -> Result<(), diesel::result::Error> {
    use schema::account_transactions::dsl::*;

    let chunks = get_chunks(item_to_insert.len(), AccountTransaction::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::account_transactions::table)
                .values(&item_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, account_address))
                .do_nothing(),
            None,
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
        let mut conn = self.get_conn();
        // get velor_coin info for supply tracking
        // TODO: This only needs to be fetched once. Need to persist somehow
        let maybe_velor_coin_info = &CoinInfoQuery::get_by_coin_type(
            VelorCoinType::type_tag().to_canonical_string(),
            &mut conn,
        )
        .unwrap();

        let mut all_coin_activities = vec![];
        let mut all_coin_balances = vec![];
        let mut all_coin_infos: HashMap<String, CoinInfo> = HashMap::new();
        let mut all_current_coin_balances: HashMap<CurrentCoinBalancePK, CurrentCoinBalance> =
            HashMap::new();
        let mut all_coin_supply = vec![];

        let mut account_transactions = HashMap::new();

        for txn in &transactions {
            let (
                mut coin_activities,
                mut coin_balances,
                coin_infos,
                current_coin_balances,
                mut coin_supply,
            ) = CoinActivity::from_transaction(txn, maybe_velor_coin_info);
            all_coin_activities.append(&mut coin_activities);
            all_coin_balances.append(&mut coin_balances);
            all_coin_supply.append(&mut coin_supply);
            // For coin infos, we only want to keep the first version, so insert only if key is not present already
            for (key, value) in coin_infos {
                all_coin_infos.entry(key).or_insert(value);
            }
            all_current_coin_balances.extend(current_coin_balances);

            account_transactions.extend(AccountTransaction::from_transaction(txn).unwrap());
        }
        let mut all_coin_infos = all_coin_infos.into_values().collect::<Vec<CoinInfo>>();
        let mut all_current_coin_balances = all_current_coin_balances
            .into_values()
            .collect::<Vec<CurrentCoinBalance>>();
        let mut account_transactions = account_transactions
            .into_values()
            .collect::<Vec<AccountTransaction>>();

        // Sort by PK
        all_coin_infos.sort_by(|a, b| a.coin_type.cmp(&b.coin_type));
        all_current_coin_balances.sort_by(|a, b| {
            (&a.owner_address, &a.coin_type).cmp(&(&b.owner_address, &b.coin_type))
        });
        account_transactions.sort_by(|a, b| {
            (&a.transaction_version, &a.account_address)
                .cmp(&(&b.transaction_version, &b.account_address))
        });

        let tx_result = insert_to_db(
            &mut conn,
            self.name(),
            start_version,
            end_version,
            all_coin_activities,
            all_coin_infos,
            all_coin_balances,
            all_current_coin_balances,
            all_coin_supply,
            account_transactions,
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
