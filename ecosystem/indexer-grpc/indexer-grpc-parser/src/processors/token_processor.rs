// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::processor_trait::{ProcessingResult, ProcessorTrait};
use crate::{
    models::token_models::{
        ans_lookup::{CurrentAnsLookup, CurrentAnsLookupPK},
        collection_datas::{CollectionData, CurrentCollectionData},
        token_activities::TokenActivity,
        token_claims::CurrentTokenPendingClaim,
        token_datas::{CurrentTokenData, TokenData},
        token_ownerships::{CurrentTokenOwnership, TokenOwnership},
        tokens::{
            CurrentTokenOwnershipPK, CurrentTokenPendingClaimPK, TableMetadataForToken, Token,
            TokenDataIdHash,
        },
    },
    schema,
    utils::database::{
        clean_data_for_db, execute_with_better_error, get_chunks, PgDbPool, PgPoolConnection,
    },
};
use anyhow::bail;
use aptos_logger::error;
use aptos_protos::transaction::testing1::v1::Transaction;
use async_trait::async_trait;
use diesel::{pg::upsert::excluded, result::Error, ExpressionMethods, PgConnection};
use field_count::FieldCount;
use std::{collections::HashMap, fmt::Debug};

pub const NAME: &str = "token_processor";
pub struct TokenTransactionProcessor {
    connection_pool: PgDbPool,
    ans_contract_address: Option<String>,
}

impl TokenTransactionProcessor {
    pub fn new(connection_pool: PgDbPool, ans_contract_address: Option<String>) -> Self {
        aptos_logger::info!(
            ans_contract_address = ans_contract_address,
            "init TokenTransactionProcessor"
        );
        Self {
            connection_pool,
            ans_contract_address,
        }
    }
}

impl Debug for TokenTransactionProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = &self.connection_pool.state();
        write!(
            f,
            "TokenTransactionProcessor {{ connections: {:?}  idle_connections: {:?} }}",
            state.connections, state.idle_connections
        )
    }
}

fn insert_to_db_impl(
    conn: &mut PgConnection,
    basic_token_transaction_lists: (&[Token], &[TokenOwnership], &[TokenData], &[CollectionData]),
    basic_token_current_lists: (
        &[CurrentTokenOwnership],
        &[CurrentTokenData],
        &[CurrentCollectionData],
    ),
    token_activities: &[TokenActivity],
    current_token_claims: &[CurrentTokenPendingClaim],
    current_ans_lookups: &[CurrentAnsLookup],
) -> Result<(), diesel::result::Error> {
    let (tokens, token_ownerships, token_datas, collection_datas) = basic_token_transaction_lists;
    let (current_token_ownerships, current_token_datas, current_collection_datas) =
        basic_token_current_lists;
    insert_tokens(conn, tokens)?;
    insert_token_datas(conn, token_datas)?;
    insert_token_ownerships(conn, token_ownerships)?;
    insert_collection_datas(conn, collection_datas)?;
    insert_current_token_ownerships(conn, current_token_ownerships)?;
    insert_current_token_datas(conn, current_token_datas)?;
    insert_current_collection_datas(conn, current_collection_datas)?;
    insert_token_activities(conn, token_activities)?;
    insert_current_token_claims(conn, current_token_claims)?;
    insert_current_ans_lookups(conn, current_ans_lookups)?;
    Ok(())
}

fn insert_to_db(
    conn: &mut PgPoolConnection,
    name: &'static str,
    start_version: u64,
    end_version: u64,
    basic_token_transaction_lists: (
        Vec<Token>,
        Vec<TokenOwnership>,
        Vec<TokenData>,
        Vec<CollectionData>,
    ),
    basic_token_current_lists: (
        Vec<CurrentTokenOwnership>,
        Vec<CurrentTokenData>,
        Vec<CurrentCollectionData>,
    ),
    token_activities: Vec<TokenActivity>,
    current_token_claims: Vec<CurrentTokenPendingClaim>,
    current_ans_lookups: Vec<CurrentAnsLookup>,
) -> Result<(), diesel::result::Error> {
    aptos_logger::trace!(
        name = name,
        start_version = start_version,
        end_version = end_version,
        "Inserting to db",
    );
    let (tokens, token_ownerships, token_datas, collection_datas) = basic_token_transaction_lists;
    let (current_token_ownerships, current_token_datas, current_collection_datas) =
        basic_token_current_lists;
    match conn
        .build_transaction()
        .read_write()
        .run::<_, Error, _>(|pg_conn| {
            insert_to_db_impl(
                pg_conn,
                (&tokens, &token_ownerships, &token_datas, &collection_datas),
                (
                    &current_token_ownerships,
                    &current_token_datas,
                    &current_collection_datas,
                ),
                &token_activities,
                &current_token_claims,
                &current_ans_lookups,
            )
        }) {
        Ok(_) => Ok(()),
        Err(_) => conn
            .build_transaction()
            .read_write()
            .run::<_, Error, _>(|pg_conn| {
                let tokens = clean_data_for_db(tokens, true);
                let token_datas = clean_data_for_db(token_datas, true);
                let token_ownerships = clean_data_for_db(token_ownerships, true);
                let collection_datas = clean_data_for_db(collection_datas, true);
                let current_token_ownerships = clean_data_for_db(current_token_ownerships, true);
                let current_token_datas = clean_data_for_db(current_token_datas, true);
                let current_collection_datas = clean_data_for_db(current_collection_datas, true);
                let token_activities = clean_data_for_db(token_activities, true);
                let current_token_claims = clean_data_for_db(current_token_claims, true);
                let current_ans_lookups = clean_data_for_db(current_ans_lookups, true);

                insert_to_db_impl(
                    pg_conn,
                    (&tokens, &token_ownerships, &token_datas, &collection_datas),
                    (
                        &current_token_ownerships,
                        &current_token_datas,
                        &current_collection_datas,
                    ),
                    &token_activities,
                    &current_token_claims,
                    &current_ans_lookups,
                )
            }),
    }
}

fn insert_tokens(
    conn: &mut PgConnection,
    tokens_to_insert: &[Token],
) -> Result<(), diesel::result::Error> {
    use schema::tokens::dsl::*;

    let chunks = get_chunks(tokens_to_insert.len(), Token::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::tokens::table)
                .values(&tokens_to_insert[start_ind..end_ind])
                .on_conflict((token_data_id_hash, property_version, transaction_version))
                .do_update()
                .set((
                    token_properties.eq(excluded(token_properties)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
            None,
        )?;
    }
    Ok(())
}

fn insert_token_ownerships(
    conn: &mut PgConnection,
    token_ownerships_to_insert: &[TokenOwnership],
) -> Result<(), diesel::result::Error> {
    use schema::token_ownerships::dsl::*;

    let chunks = get_chunks(
        token_ownerships_to_insert.len(),
        TokenOwnership::field_count(),
    );
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::token_ownerships::table)
                .values(&token_ownerships_to_insert[start_ind..end_ind])
                .on_conflict((
                    token_data_id_hash,
                    property_version,
                    transaction_version,
                    table_handle,
                ))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_token_datas(
    conn: &mut PgConnection,
    token_datas_to_insert: &[TokenData],
) -> Result<(), diesel::result::Error> {
    use schema::token_datas::dsl::*;

    let chunks = get_chunks(token_datas_to_insert.len(), TokenData::field_count());
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::token_datas::table)
                .values(&token_datas_to_insert[start_ind..end_ind])
                .on_conflict((token_data_id_hash, transaction_version))
                .do_update()
                .set((
                    default_properties.eq(excluded(default_properties)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
            None,
        )?;
    }
    Ok(())
}

fn insert_collection_datas(
    conn: &mut PgConnection,
    collection_datas_to_insert: &[CollectionData],
) -> Result<(), diesel::result::Error> {
    use schema::collection_datas::dsl::*;

    let chunks = get_chunks(
        collection_datas_to_insert.len(),
        CollectionData::field_count(),
    );
    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::collection_datas::table)
                .values(&collection_datas_to_insert[start_ind..end_ind])
                .on_conflict((collection_data_id_hash, transaction_version))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_current_token_ownerships(
    conn: &mut PgConnection,
    items_to_insert: &[CurrentTokenOwnership],
) -> Result<(), diesel::result::Error> {
    use schema::current_token_ownerships::dsl::*;

    let chunks = get_chunks(items_to_insert.len(), CurrentTokenOwnership::field_count());

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::current_token_ownerships::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((token_data_id_hash, property_version, owner_address))
                .do_update()
                .set((
                    creator_address.eq(excluded(creator_address)),
                    collection_name.eq(excluded(collection_name)),
                    name.eq(excluded(name)),
                    amount.eq(excluded(amount)),
                    token_properties.eq(excluded(token_properties)),
                    last_transaction_version.eq(excluded(last_transaction_version)),
                    collection_data_id_hash.eq(excluded(collection_data_id_hash)),
                    table_type.eq(excluded(table_type)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
            Some(" WHERE current_token_ownerships.last_transaction_version <= excluded.last_transaction_version "),
        )?;
    }
    Ok(())
}

fn insert_current_token_datas(
    conn: &mut PgConnection,
    items_to_insert: &[CurrentTokenData],
) -> Result<(), diesel::result::Error> {
    use schema::current_token_datas::dsl::*;

    let chunks = get_chunks(items_to_insert.len(), CurrentTokenData::field_count());

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::current_token_datas::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict(token_data_id_hash)
                .do_update()
                .set((
                    creator_address.eq(excluded(creator_address)),
                    collection_name.eq(excluded(collection_name)),
                    name.eq(excluded(name)),
                    maximum.eq(excluded(maximum)),
                    supply.eq(excluded(supply)),
                    largest_property_version.eq(excluded(largest_property_version)),
                    metadata_uri.eq(excluded(metadata_uri)),
                    payee_address.eq(excluded(payee_address)),
                    royalty_points_numerator.eq(excluded(royalty_points_numerator)),
                    royalty_points_denominator.eq(excluded(royalty_points_denominator)),
                    maximum_mutable.eq(excluded(maximum_mutable)),
                    uri_mutable.eq(excluded(uri_mutable)),
                    description_mutable.eq(excluded(description_mutable)),
                    properties_mutable.eq(excluded(properties_mutable)),
                    royalty_mutable.eq(excluded(royalty_mutable)),
                    default_properties.eq(excluded(default_properties)),
                    last_transaction_version.eq(excluded(last_transaction_version)),
                    collection_data_id_hash.eq(excluded(collection_data_id_hash)),
                    description.eq(excluded(description)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
            Some(" WHERE current_token_datas.last_transaction_version <= excluded.last_transaction_version "),
        )?;
    }
    Ok(())
}

fn insert_current_collection_datas(
    conn: &mut PgConnection,
    items_to_insert: &[CurrentCollectionData],
) -> Result<(), diesel::result::Error> {
    use schema::current_collection_datas::dsl::*;

    let chunks = get_chunks(items_to_insert.len(), CurrentCollectionData::field_count());

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::current_collection_datas::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict(collection_data_id_hash)
                .do_update()
                .set((
                    creator_address.eq(excluded(creator_address)),
                    collection_name.eq(excluded(collection_name)),
                    description.eq(excluded(description)),
                    metadata_uri.eq(excluded(metadata_uri)),
                    supply.eq(excluded(supply)),
                    maximum.eq(excluded(maximum)),
                    maximum_mutable.eq(excluded(maximum_mutable)),
                    uri_mutable.eq(excluded(uri_mutable)),
                    description_mutable.eq(excluded(description_mutable)),
                    last_transaction_version.eq(excluded(last_transaction_version)),
                    table_handle.eq(excluded(table_handle)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
            Some(" WHERE current_collection_datas.last_transaction_version <= excluded.last_transaction_version "),
        )?;
    }
    Ok(())
}

fn insert_token_activities(
    conn: &mut PgConnection,
    items_to_insert: &[TokenActivity],
) -> Result<(), diesel::result::Error> {
    use schema::token_activities::dsl::*;

    let chunks = get_chunks(items_to_insert.len(), TokenActivity::field_count());

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::token_activities::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((
                    transaction_version,
                    event_account_address,
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
fn insert_current_token_claims(
    conn: &mut PgConnection,
    items_to_insert: &[CurrentTokenPendingClaim],
) -> Result<(), diesel::result::Error> {
    use schema::current_token_pending_claims::dsl::*;

    let chunks = get_chunks(
        items_to_insert.len(),
        CurrentTokenPendingClaim::field_count(),
    );

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::current_token_pending_claims::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((
                    token_data_id_hash, property_version, from_address, to_address
                ))
                .do_update()
                .set((
                    collection_data_id_hash.eq(excluded(collection_data_id_hash)),
                    creator_address.eq(excluded(creator_address)),
                    collection_name.eq(excluded(collection_name)),
                    name.eq(excluded(name)),
                    amount.eq(excluded(amount)),
                    table_handle.eq(excluded(table_handle)),
                    last_transaction_version.eq(excluded(last_transaction_version)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
            Some(" WHERE current_token_pending_claims.last_transaction_version <= excluded.last_transaction_version "),
        )?;
    }
    Ok(())
}

fn insert_current_ans_lookups(
    conn: &mut PgConnection,
    items_to_insert: &[CurrentAnsLookup],
) -> Result<(), diesel::result::Error> {
    use schema::current_ans_lookup::dsl::*;

    let chunks = get_chunks(items_to_insert.len(), CurrentAnsLookup::field_count());

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::current_ans_lookup::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((domain, subdomain))
                .do_update()
                .set((
                    registered_address.eq(excluded(registered_address)),
                    expiration_timestamp.eq(excluded(expiration_timestamp)),
                    last_transaction_version.eq(excluded(last_transaction_version)),
                    inserted_at.eq(excluded(inserted_at)),
                    token_name.eq(excluded(token_name)),
                )),
                Some(" WHERE current_ans_lookup.last_transaction_version <= excluded.last_transaction_version "),
            )?;
    }
    Ok(())
}

#[async_trait]
impl ProcessorTrait for TokenTransactionProcessor {
    fn name(&self) -> &'static str {
        NAME
    }

    async fn process_transactions(
        &self,
        transactions: Vec<Transaction>,
        start_version: u64,
        end_version: u64,
    ) -> anyhow::Result<ProcessingResult> {
        let mut conn = self.get_conn();

        // First get all token related table metadata from the batch of transactions. This is in case
        // an earlier transaction has metadata (in resources) that's missing from a later transaction.
        let table_handle_to_owner =
            TableMetadataForToken::get_table_handle_to_owner_from_transactions(&transactions);

        let mut all_tokens = vec![];
        let mut all_token_ownerships = vec![];
        let mut all_token_datas = vec![];
        let mut all_collection_datas = vec![];
        let mut all_token_activities = vec![];

        // Hashmap key will be the PK of the table, we do not want to send duplicates writes to the db within a batch
        let mut all_current_token_ownerships: HashMap<
            CurrentTokenOwnershipPK,
            CurrentTokenOwnership,
        > = HashMap::new();
        let mut all_current_token_datas: HashMap<TokenDataIdHash, CurrentTokenData> =
            HashMap::new();
        let mut all_current_collection_datas: HashMap<TokenDataIdHash, CurrentCollectionData> =
            HashMap::new();
        let mut all_current_token_claims: HashMap<
            CurrentTokenPendingClaimPK,
            CurrentTokenPendingClaim,
        > = HashMap::new();
        let mut all_current_ans_lookups: HashMap<CurrentAnsLookupPK, CurrentAnsLookup> =
            HashMap::new();

        for txn in transactions {
            let (
                mut tokens,
                mut token_ownerships,
                mut token_datas,
                mut collection_datas,
                current_token_ownerships,
                current_token_datas,
                current_collection_datas,
                current_token_claims,
            ) = Token::from_transaction(&txn, &table_handle_to_owner, &mut conn);
            all_tokens.append(&mut tokens);
            all_token_ownerships.append(&mut token_ownerships);
            all_token_datas.append(&mut token_datas);
            all_collection_datas.append(&mut collection_datas);
            // Given versions will always be increasing here (within a single batch), we can just override current values
            all_current_token_ownerships.extend(current_token_ownerships);
            all_current_token_datas.extend(current_token_datas);
            all_current_collection_datas.extend(current_collection_datas);

            // Track token activities
            let mut activities = TokenActivity::from_transaction(&txn);
            all_token_activities.append(&mut activities);

            // claims
            all_current_token_claims.extend(current_token_claims);

            // ANS lookups
            let current_ans_lookups =
                CurrentAnsLookup::from_transaction(&txn, self.ans_contract_address.clone());
            all_current_ans_lookups.extend(current_ans_lookups);
        }

        // Getting list of values and sorting by pk in order to avoid postgres deadlock since we're doing multi threaded db writes
        let mut all_current_token_ownerships = all_current_token_ownerships
            .into_values()
            .collect::<Vec<CurrentTokenOwnership>>();
        let mut all_current_token_datas = all_current_token_datas
            .into_values()
            .collect::<Vec<CurrentTokenData>>();
        let mut all_current_collection_datas = all_current_collection_datas
            .into_values()
            .collect::<Vec<CurrentCollectionData>>();
        let mut all_current_token_claims = all_current_token_claims
            .into_values()
            .collect::<Vec<CurrentTokenPendingClaim>>();

        // Sort by PK
        all_current_token_ownerships.sort_by(|a, b| {
            (&a.token_data_id_hash, &a.property_version, &a.owner_address).cmp(&(
                &b.token_data_id_hash,
                &b.property_version,
                &b.owner_address,
            ))
        });
        all_current_token_datas.sort_by(|a, b| a.token_data_id_hash.cmp(&b.token_data_id_hash));
        all_current_collection_datas
            .sort_by(|a, b| a.collection_data_id_hash.cmp(&b.collection_data_id_hash));
        all_current_token_claims.sort_by(|a, b| {
            (
                &a.token_data_id_hash,
                &a.property_version,
                &a.from_address,
                &a.to_address,
            )
                .cmp(&(
                    &b.token_data_id_hash,
                    &b.property_version,
                    &b.from_address,
                    &a.to_address,
                ))
        });
        // Sort ans lookup values for postgres insert
        let mut all_current_ans_lookups = all_current_ans_lookups
            .into_values()
            .collect::<Vec<CurrentAnsLookup>>();
        all_current_ans_lookups
            .sort_by(|a, b| a.domain.cmp(&b.domain).then(a.subdomain.cmp(&b.subdomain)));

        let tx_result = insert_to_db(
            &mut conn,
            self.name(),
            start_version,
            end_version,
            (
                all_tokens,
                all_token_ownerships,
                all_token_datas,
                all_collection_datas,
            ),
            (
                all_current_token_ownerships,
                all_current_token_datas,
                all_current_collection_datas,
            ),
            all_token_activities,
            all_current_token_claims,
            all_current_ans_lookups,
        );
        match tx_result {
            Ok(_) => Ok((start_version, end_version)),
            Err(e) => {
                error!(
                    start_version = start_version,
                    end_version = end_version,
                    processor_name = self.name(),
                    error = ?e,
                    "[Parser] Error inserting transactions to db",
                );
                bail!(e)
            },
        }
    }

    fn connection_pool(&self) -> &PgDbPool {
        &self.connection_pool
    }
}
