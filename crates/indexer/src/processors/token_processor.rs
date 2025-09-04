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
    models::{
        coin_models::{
            coin_activities::MAX_ENTRY_FUNCTION_LENGTH,
            v2_fungible_asset_utils::{
                FungibleAssetMetadata, FungibleAssetStore, FungibleAssetSupply,
            },
        },
        token_models::{
            ans_lookup::{CurrentAnsLookup, CurrentAnsLookupPK},
            collection_datas::{CollectionData, CurrentCollectionData},
            nft_points::NftPoints,
            token_activities::TokenActivity,
            token_claims::CurrentTokenPendingClaim,
            token_datas::{CurrentTokenData, TokenData},
            token_ownerships::{CurrentTokenOwnership, TokenOwnership},
            tokens::{
                CurrentTokenOwnershipPK, CurrentTokenPendingClaimPK, TableHandleToOwner,
                TableMetadataForToken, Token, TokenDataIdHash,
            },
            v2_collections::{CollectionV2, CurrentCollectionV2, CurrentCollectionV2PK},
            v2_token_activities::TokenActivityV2,
            v2_token_datas::{CurrentTokenDataV2, CurrentTokenDataV2PK, TokenDataV2},
            v2_token_metadata::{CurrentTokenV2Metadata, CurrentTokenV2MetadataPK},
            v2_token_ownerships::{
                CurrentTokenOwnershipV2, CurrentTokenOwnershipV2PK, NFTOwnershipV2,
                TokenOwnershipV2,
            },
            v2_token_utils::{
                VelorCollection, BurnEvent, FixedSupply, ObjectWithMetadata, PropertyMap, TokenV2,
                TokenV2AggregatedData, TokenV2AggregatedDataMapping, TokenV2Burned, TransferEvent,
                UnlimitedSupply,
            },
        },
    },
    schema,
    util::{parse_timestamp, standardize_address, truncate_str},
};
use velor_api_types::{Transaction, TransactionPayload, WriteSetChange};
use async_trait::async_trait;
use diesel::{pg::upsert::excluded, result::Error, ExpressionMethods, PgConnection};
use field_count::FieldCount;
use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
};

pub const NAME: &str = "token_processor";
pub struct TokenTransactionProcessor {
    connection_pool: PgDbPool,
    ans_contract_address: Option<String>,
    nft_points_contract: Option<String>,
}

impl TokenTransactionProcessor {
    pub fn new(
        connection_pool: PgDbPool,
        ans_contract_address: Option<String>,
        nft_points_contract: Option<String>,
    ) -> Self {
        velor_logger::info!(
            ans_contract_address = ans_contract_address,
            "init TokenTransactionProcessor"
        );
        Self {
            connection_pool,
            ans_contract_address,
            nft_points_contract,
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
    nft_points: &[NftPoints],
    (
        collections_v2,
        token_datas_v2,
        token_ownerships_v2,
        current_collections_v2,
        current_token_datas_v2,
        current_token_ownerships_v2,
        token_activities_v2,
        current_token_v2_metadata,
    ): (
        &[CollectionV2],
        &[TokenDataV2],
        &[TokenOwnershipV2],
        &[CurrentCollectionV2],
        &[CurrentTokenDataV2],
        &[CurrentTokenOwnershipV2],
        &[TokenActivityV2],
        &[CurrentTokenV2Metadata],
    ),
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
    insert_nft_points(conn, nft_points)?;
    insert_collections_v2(conn, collections_v2)?;
    insert_token_datas_v2(conn, token_datas_v2)?;
    insert_token_ownerships_v2(conn, token_ownerships_v2)?;
    insert_current_collections_v2(conn, current_collections_v2)?;
    insert_current_token_datas_v2(conn, current_token_datas_v2)?;
    insert_current_token_ownerships_v2(conn, current_token_ownerships_v2)?;
    insert_token_activities_v2(conn, token_activities_v2)?;
    insert_current_token_v2_metadatas(conn, current_token_v2_metadata)?;
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
    nft_points: Vec<NftPoints>,
    (
        collections_v2,
        token_datas_v2,
        token_ownerships_v2,
        current_collections_v2,
        current_token_datas_v2,
        current_token_ownerships_v2,
        token_activities_v2,
        current_token_v2_metadata,
    ): (
        Vec<CollectionV2>,
        Vec<TokenDataV2>,
        Vec<TokenOwnershipV2>,
        Vec<CurrentCollectionV2>,
        Vec<CurrentTokenDataV2>,
        Vec<CurrentTokenOwnershipV2>,
        Vec<TokenActivityV2>,
        Vec<CurrentTokenV2Metadata>,
    ),
) -> Result<(), diesel::result::Error> {
    velor_logger::trace!(
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
                &nft_points,
                (
                    &collections_v2,
                    &token_datas_v2,
                    &token_ownerships_v2,
                    &current_collections_v2,
                    &current_token_datas_v2,
                    &current_token_ownerships_v2,
                    &token_activities_v2,
                    &current_token_v2_metadata,
                ),
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
                let nft_points = clean_data_for_db(nft_points, true);
                let collections_v2 = clean_data_for_db(collections_v2, true);
                let token_datas_v2 = clean_data_for_db(token_datas_v2, true);
                let token_ownerships_v2 = clean_data_for_db(token_ownerships_v2, true);
                let current_collections_v2 = clean_data_for_db(current_collections_v2, true);
                let current_token_datas_v2 = clean_data_for_db(current_token_datas_v2, true);
                let current_token_ownerships_v2 =
                    clean_data_for_db(current_token_ownerships_v2, true);
                let token_activities_v2 = clean_data_for_db(token_activities_v2, true);
                let current_token_v2_metadata = clean_data_for_db(current_token_v2_metadata, true);

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
                    &nft_points,
                    (
                        &collections_v2,
                        &token_datas_v2,
                        &token_ownerships_v2,
                        &current_collections_v2,
                        &current_token_datas_v2,
                        &current_token_ownerships_v2,
                        &token_activities_v2,
                        &current_token_v2_metadata,
                    ),
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
                    token_data_id.eq(excluded(token_data_id)),
                    collection_id.eq(excluded(collection_id)),
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

fn insert_nft_points(
    conn: &mut PgConnection,
    items_to_insert: &[NftPoints],
) -> Result<(), diesel::result::Error> {
    use schema::nft_points::dsl::*;

    let chunks = get_chunks(items_to_insert.len(), NftPoints::field_count());

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::nft_points::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict(transaction_version)
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_collections_v2(
    conn: &mut PgConnection,
    items_to_insert: &[CollectionV2],
) -> Result<(), diesel::result::Error> {
    use schema::collections_v2::dsl::*;

    let chunks = get_chunks(items_to_insert.len(), CollectionV2::field_count());

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::collections_v2::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, write_set_change_index))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_token_datas_v2(
    conn: &mut PgConnection,
    items_to_insert: &[TokenDataV2],
) -> Result<(), diesel::result::Error> {
    use schema::token_datas_v2::dsl::*;

    let chunks = get_chunks(items_to_insert.len(), TokenDataV2::field_count());

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::token_datas_v2::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, write_set_change_index))
                .do_update()
                .set((
                    inserted_at.eq(excluded(inserted_at)),
                    decimals.eq(excluded(decimals)),
                )),
            None,
        )?;
    }
    Ok(())
}

fn insert_token_ownerships_v2(
    conn: &mut PgConnection,
    items_to_insert: &[TokenOwnershipV2],
) -> Result<(), diesel::result::Error> {
    use schema::token_ownerships_v2::dsl::*;

    let chunks = get_chunks(items_to_insert.len(), TokenOwnershipV2::field_count());

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::token_ownerships_v2::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, write_set_change_index))
                .do_update()
                .set((
                    token_data_id.eq(excluded(token_data_id)),
                    property_version_v1.eq(excluded(property_version_v1)),
                    owner_address.eq(excluded(owner_address)),
                    storage_id.eq(excluded(storage_id)),
                    amount.eq(excluded(amount)),
                    table_type_v1.eq(excluded(table_type_v1)),
                    token_properties_mutated_v1.eq(excluded(token_properties_mutated_v1)),
                    is_soulbound_v2.eq(excluded(is_soulbound_v2)),
                    token_standard.eq(excluded(token_standard)),
                    is_fungible_v2.eq(excluded(is_fungible_v2)),
                    transaction_timestamp.eq(excluded(transaction_timestamp)),
                    inserted_at.eq(excluded(inserted_at)),
                    non_transferrable_by_owner.eq(excluded(non_transferrable_by_owner)),
                )),
            None,
        )?;
    }
    Ok(())
}

fn insert_current_collections_v2(
    conn: &mut PgConnection,
    items_to_insert: &[CurrentCollectionV2],
) -> Result<(), diesel::result::Error> {
    use schema::current_collections_v2::dsl::*;

    let chunks = get_chunks(items_to_insert.len(), CurrentCollectionV2::field_count());

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::current_collections_v2::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict(collection_id)
                .do_update()
                .set((
                    creator_address.eq(excluded(creator_address)),
                    collection_name.eq(excluded(collection_name)),
                    description.eq(excluded(description)),
                    uri.eq(excluded(uri)),
                    current_supply.eq(excluded(current_supply)),
                    max_supply.eq(excluded(max_supply)),
                    total_minted_v2.eq(excluded(total_minted_v2)),
                    mutable_description.eq(excluded(mutable_description)),
                    mutable_uri.eq(excluded(mutable_uri)),
                    table_handle_v1.eq(excluded(table_handle_v1)),
                    token_standard.eq(excluded(token_standard)),
                    last_transaction_version.eq(excluded(last_transaction_version)),
                    last_transaction_timestamp.eq(excluded(last_transaction_timestamp)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
            Some(" WHERE current_collections_v2.last_transaction_version <= excluded.last_transaction_version "),
        )?;
    }
    Ok(())
}

fn insert_current_token_datas_v2(
    conn: &mut PgConnection,
    items_to_insert: &[CurrentTokenDataV2],
) -> Result<(), diesel::result::Error> {
    use schema::current_token_datas_v2::dsl::*;

    let chunks = get_chunks(items_to_insert.len(), CurrentTokenDataV2::field_count());

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::current_token_datas_v2::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict(token_data_id)
                .do_update()
                .set((
                    collection_id.eq(excluded(collection_id)),
                    token_name.eq(excluded(token_name)),
                    maximum.eq(excluded(maximum)),
                    supply.eq(excluded(supply)),
                    largest_property_version_v1.eq(excluded(largest_property_version_v1)),
                    token_uri.eq(excluded(token_uri)),
                    description.eq(excluded(description)),
                    token_properties.eq(excluded(token_properties)),
                    token_standard.eq(excluded(token_standard)),
                    is_fungible_v2.eq(excluded(is_fungible_v2)),
                    last_transaction_version.eq(excluded(last_transaction_version)),
                    last_transaction_timestamp.eq(excluded(last_transaction_timestamp)),
                    inserted_at.eq(excluded(inserted_at)),
                    decimals.eq(excluded(decimals)),
                )),
            Some(" WHERE current_token_datas_v2.last_transaction_version <= excluded.last_transaction_version "),
        )?;
    }
    Ok(())
}

fn insert_current_token_ownerships_v2(
    conn: &mut PgConnection,
    items_to_insert: &[CurrentTokenOwnershipV2],
) -> Result<(), diesel::result::Error> {
    use schema::current_token_ownerships_v2::dsl::*;

    let chunks = get_chunks(
        items_to_insert.len(),
        CurrentTokenOwnershipV2::field_count(),
    );

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::current_token_ownerships_v2::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((token_data_id, property_version_v1, owner_address, storage_id))
                .do_update()
                .set((
                    amount.eq(excluded(amount)),
                    table_type_v1.eq(excluded(table_type_v1)),
                    token_properties_mutated_v1.eq(excluded(token_properties_mutated_v1)),
                    is_soulbound_v2.eq(excluded(is_soulbound_v2)),
                    token_standard.eq(excluded(token_standard)),
                    is_fungible_v2.eq(excluded(is_fungible_v2)),
                    last_transaction_version.eq(excluded(last_transaction_version)),
                    last_transaction_timestamp.eq(excluded(last_transaction_timestamp)),
                    inserted_at.eq(excluded(inserted_at)),
                    non_transferrable_by_owner.eq(excluded(non_transferrable_by_owner)),
                )),
            Some(" WHERE current_token_ownerships_v2.last_transaction_version <= excluded.last_transaction_version "),
        )?;
    }
    Ok(())
}

fn insert_token_activities_v2(
    conn: &mut PgConnection,
    items_to_insert: &[TokenActivityV2],
) -> Result<(), diesel::result::Error> {
    use schema::token_activities_v2::dsl::*;

    let chunks = get_chunks(items_to_insert.len(), TokenActivityV2::field_count());

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::token_activities_v2::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((transaction_version, event_index))
                .do_nothing(),
            None,
        )?;
    }
    Ok(())
}

fn insert_current_token_v2_metadatas(
    conn: &mut PgConnection,
    items_to_insert: &[CurrentTokenV2Metadata],
) -> Result<(), diesel::result::Error> {
    use schema::current_token_v2_metadata::dsl::*;

    let chunks = get_chunks(items_to_insert.len(), CurrentTokenV2Metadata::field_count());

    for (start_ind, end_ind) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::current_token_v2_metadata::table)
                .values(&items_to_insert[start_ind..end_ind])
                .on_conflict((object_address, resource_type))
                .do_update()
                .set((
                    data.eq(excluded(data)),
                    state_key_hash.eq(excluded(state_key_hash)),
                    last_transaction_version.eq(excluded(last_transaction_version)),
                    inserted_at.eq(excluded(inserted_at)),
                )),
            Some(" WHERE current_token_v2_metadata.last_transaction_version <= excluded.last_transaction_version "),
        )?;
    }
    Ok(())
}

#[async_trait]
impl TransactionProcessor for TokenTransactionProcessor {
    fn name(&self) -> &'static str {
        NAME
    }

    async fn process_transactions(
        &self,
        transactions: Vec<Transaction>,
        start_version: u64,
        end_version: u64,
    ) -> Result<ProcessingResult, TransactionProcessingError> {
        let mut conn = self.get_conn();

        // First get all token related table metadata from the batch of transactions. This is in case
        // an earlier transaction has metadata (in resources) that's missing from a later transaction.
        let table_handle_to_owner =
            TableMetadataForToken::get_table_handle_to_owner_from_transactions(&transactions);

        // Token V1 only, this section will be deprecated soon
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

        // This is likely temporary
        let mut all_nft_points = vec![];

        for txn in &transactions {
            let (
                mut tokens,
                mut token_ownerships,
                mut token_datas,
                mut collection_datas,
                current_token_ownerships,
                current_token_datas,
                current_collection_datas,
                current_token_claims,
            ) = Token::from_transaction(txn, &table_handle_to_owner, &mut conn);
            all_tokens.append(&mut tokens);
            all_token_ownerships.append(&mut token_ownerships);
            all_token_datas.append(&mut token_datas);
            all_collection_datas.append(&mut collection_datas);
            // Given versions will always be increasing here (within a single batch), we can just override current values
            all_current_token_ownerships.extend(current_token_ownerships);
            all_current_token_datas.extend(current_token_datas);
            all_current_collection_datas.extend(current_collection_datas);

            // Track token activities
            let mut activities = TokenActivity::from_transaction(txn);
            all_token_activities.append(&mut activities);

            // claims
            all_current_token_claims.extend(current_token_claims);

            // ANS lookups
            let current_ans_lookups =
                CurrentAnsLookup::from_transaction(txn, self.ans_contract_address.clone());
            all_current_ans_lookups.extend(current_ans_lookups);

            // NFT points
            let nft_points_txn = NftPoints::from_transaction(txn, self.nft_points_contract.clone());
            if let Some(nft_points) = nft_points_txn {
                all_nft_points.push(nft_points);
            }
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

        // Token V2 processing which includes token v1
        let (
            collections_v2,
            token_datas_v2,
            token_ownerships_v2,
            current_collections_v2,
            current_token_ownerships_v2,
            current_token_datas_v2,
            token_activities_v2,
            current_token_v2_metadata,
        ) = parse_v2_token(&transactions, &table_handle_to_owner, &mut conn);

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
            all_nft_points,
            // Token V2 stuff which will token v1 tables above
            (
                collections_v2,
                token_datas_v2,
                token_ownerships_v2,
                current_collections_v2,
                current_token_ownerships_v2,
                current_token_datas_v2,
                token_activities_v2,
                current_token_v2_metadata,
            ),
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

fn parse_v2_token(
    transactions: &[Transaction],
    table_handle_to_owner: &TableHandleToOwner,
    conn: &mut PgPoolConnection,
) -> (
    Vec<CollectionV2>,
    Vec<TokenDataV2>,
    Vec<TokenOwnershipV2>,
    Vec<CurrentCollectionV2>,
    Vec<CurrentTokenDataV2>,
    Vec<CurrentTokenOwnershipV2>,
    Vec<TokenActivityV2>,
    Vec<CurrentTokenV2Metadata>,
) {
    // Token V2 and V1 combined
    let mut collections_v2 = vec![];
    let mut token_datas_v2 = vec![];
    let mut token_ownerships_v2 = vec![];
    let mut token_activities_v2 = vec![];
    let mut current_collections_v2: HashMap<CurrentCollectionV2PK, CurrentCollectionV2> =
        HashMap::new();
    let mut current_token_datas_v2: HashMap<CurrentTokenDataV2PK, CurrentTokenDataV2> =
        HashMap::new();
    let mut current_token_ownerships_v2: HashMap<
        CurrentTokenOwnershipV2PK,
        CurrentTokenOwnershipV2,
    > = HashMap::new();
    // Tracks prior ownership in case a token gets burned
    let mut prior_nft_ownership: HashMap<String, NFTOwnershipV2> = HashMap::new();
    // Get Metadata for token v2 by object
    // We want to persist this through the entire batch so that even if a token is burned,
    // we can still get the object core metadata for it
    let mut token_v2_metadata_helper: TokenV2AggregatedDataMapping = HashMap::new();
    // Basically token properties
    let mut current_token_v2_metadata: HashMap<CurrentTokenV2MetadataPK, CurrentTokenV2Metadata> =
        HashMap::new();

    // Code above is inefficient (multiple passthroughs) so I'm approaching TokenV2 with a cleaner code structure
    for txn in transactions {
        if let Transaction::UserTransaction(user_txn) = txn {
            let txn_version = user_txn.info.version.0 as i64;
            let txn_timestamp = parse_timestamp(user_txn.timestamp.0, txn_version);
            let entry_function_id_str = match &user_txn.request.payload {
                TransactionPayload::EntryFunctionPayload(payload) => Some(truncate_str(
                    &payload.function.to_string(),
                    MAX_ENTRY_FUNCTION_LENGTH,
                )),
                _ => None,
            };
            // Get burn events for token v2 by object
            let mut tokens_burned: TokenV2Burned = HashSet::new();

            // Need to do a first pass to get all the objects
            for wsc in user_txn.info.changes.iter() {
                if let WriteSetChange::WriteResource(wr) = wsc {
                    if let Some(object) =
                        ObjectWithMetadata::from_write_resource(wr, txn_version).unwrap()
                    {
                        token_v2_metadata_helper.insert(
                            standardize_address(&wr.address.to_string()),
                            TokenV2AggregatedData {
                                velor_collection: None,
                                fixed_supply: None,
                                object,
                                unlimited_supply: None,
                                property_map: None,
                                transfer_event: None,
                                token: None,
                                fungible_asset_metadata: None,
                                fungible_asset_supply: None,
                                fungible_asset_store: None,
                            },
                        );
                    }
                }
            }

            // Need to do a second pass to get all the structs related to the object
            for wsc in user_txn.info.changes.iter() {
                if let WriteSetChange::WriteResource(wr) = wsc {
                    let address = standardize_address(&wr.address.to_string());
                    if let Some(aggregated_data) = token_v2_metadata_helper.get_mut(&address) {
                        if let Some(fixed_supply) =
                            FixedSupply::from_write_resource(wr, txn_version).unwrap()
                        {
                            aggregated_data.fixed_supply = Some(fixed_supply);
                        }
                        if let Some(unlimited_supply) =
                            UnlimitedSupply::from_write_resource(wr, txn_version).unwrap()
                        {
                            aggregated_data.unlimited_supply = Some(unlimited_supply);
                        }
                        if let Some(velor_collection) =
                            VelorCollection::from_write_resource(wr, txn_version).unwrap()
                        {
                            aggregated_data.velor_collection = Some(velor_collection);
                        }
                        if let Some(property_map) =
                            PropertyMap::from_write_resource(wr, txn_version).unwrap()
                        {
                            aggregated_data.property_map = Some(property_map);
                        }
                        if let Some(token) = TokenV2::from_write_resource(wr, txn_version).unwrap()
                        {
                            aggregated_data.token = Some(token);
                        }
                        if let Some(fungible_asset_metadata) =
                            FungibleAssetMetadata::from_write_resource(wr, txn_version).unwrap()
                        {
                            aggregated_data.fungible_asset_metadata = Some(fungible_asset_metadata);
                        }
                        if let Some(fungible_asset_supply) =
                            FungibleAssetSupply::from_write_resource(wr, txn_version).unwrap()
                        {
                            aggregated_data.fungible_asset_supply = Some(fungible_asset_supply);
                        }
                        if let Some(fungible_asset_store) =
                            FungibleAssetStore::from_write_resource(wr, txn_version).unwrap()
                        {
                            aggregated_data.fungible_asset_store = Some(fungible_asset_store);
                        }
                    }
                }
            }

            // Pass through events to get the burn events and token activities v2
            // This needs to be here because we need the metadata above for token activities
            // and burn / transfer events need to come before the next section
            for (index, event) in user_txn.events.iter().enumerate() {
                if let Some(burn_event) = BurnEvent::from_event(event, txn_version).unwrap() {
                    tokens_burned.insert(burn_event.get_token_address());
                }
                if let Some(transfer_event) = TransferEvent::from_event(event, txn_version).unwrap()
                {
                    if let Some(aggregated_data) =
                        token_v2_metadata_helper.get_mut(&transfer_event.get_object_address())
                    {
                        // we don't want index to be 0 otherwise we might have collision with write set change index
                        let index = if index == 0 {
                            user_txn.events.len()
                        } else {
                            index
                        };
                        aggregated_data.transfer_event = Some((index as i64, transfer_event));
                    }
                }
                // handling all the token v1 events
                if let Some(event) = TokenActivityV2::get_v1_from_parsed_event(
                    event,
                    txn_version,
                    txn_timestamp,
                    index as i64,
                    &entry_function_id_str,
                )
                .unwrap()
                {
                    token_activities_v2.push(event);
                }
                // handling token v2 nft events
                if let Some(event) = TokenActivityV2::get_nft_v2_from_parsed_event(
                    event,
                    txn_version,
                    txn_timestamp,
                    index as i64,
                    &entry_function_id_str,
                    &token_v2_metadata_helper,
                )
                .unwrap()
                {
                    token_activities_v2.push(event);
                }
                // handling token v2 fungible token events
                if let Some(event) = TokenActivityV2::get_ft_v2_from_parsed_event(
                    event,
                    txn_version,
                    txn_timestamp,
                    index as i64,
                    &entry_function_id_str,
                    &token_v2_metadata_helper,
                    conn,
                )
                .unwrap()
                {
                    token_activities_v2.push(event);
                }
            }

            for (index, wsc) in user_txn.info.changes.iter().enumerate() {
                let wsc_index = index as i64;
                match wsc {
                    WriteSetChange::WriteTableItem(table_item) => {
                        if let Some((collection, current_collection)) =
                            CollectionV2::get_v1_from_write_table_item(
                                table_item,
                                txn_version,
                                wsc_index,
                                txn_timestamp,
                                table_handle_to_owner,
                                conn,
                            )
                            .unwrap()
                        {
                            collections_v2.push(collection);
                            current_collections_v2.insert(
                                current_collection.collection_id.clone(),
                                current_collection,
                            );
                        }
                        if let Some((token_data, current_token_data)) =
                            TokenDataV2::get_v1_from_write_table_item(
                                table_item,
                                txn_version,
                                wsc_index,
                                txn_timestamp,
                            )
                            .unwrap()
                        {
                            token_datas_v2.push(token_data);
                            current_token_datas_v2.insert(
                                current_token_data.token_data_id.clone(),
                                current_token_data,
                            );
                        }
                        if let Some((token_ownership, current_token_ownership)) =
                            TokenOwnershipV2::get_v1_from_write_table_item(
                                table_item,
                                txn_version,
                                wsc_index,
                                txn_timestamp,
                                table_handle_to_owner,
                            )
                            .unwrap()
                        {
                            token_ownerships_v2.push(token_ownership);
                            if let Some(cto) = current_token_ownership {
                                prior_nft_ownership.insert(
                                    cto.token_data_id.clone(),
                                    NFTOwnershipV2 {
                                        token_data_id: cto.token_data_id.clone(),
                                        owner_address: cto.owner_address.clone(),
                                        is_soulbound: cto.is_soulbound_v2,
                                    },
                                );
                                current_token_ownerships_v2.insert(
                                    (
                                        cto.token_data_id.clone(),
                                        cto.property_version_v1.clone(),
                                        cto.owner_address.clone(),
                                        cto.storage_id.clone(),
                                    ),
                                    cto,
                                );
                            }
                        }
                    },
                    WriteSetChange::DeleteTableItem(table_item) => {
                        if let Some((token_ownership, current_token_ownership)) =
                            TokenOwnershipV2::get_v1_from_delete_table_item(
                                table_item,
                                txn_version,
                                wsc_index,
                                txn_timestamp,
                                table_handle_to_owner,
                            )
                            .unwrap()
                        {
                            token_ownerships_v2.push(token_ownership);
                            if let Some(cto) = current_token_ownership {
                                prior_nft_ownership.insert(
                                    cto.token_data_id.clone(),
                                    NFTOwnershipV2 {
                                        token_data_id: cto.token_data_id.clone(),
                                        owner_address: cto.owner_address.clone(),
                                        is_soulbound: cto.is_soulbound_v2,
                                    },
                                );
                                current_token_ownerships_v2.insert(
                                    (
                                        cto.token_data_id.clone(),
                                        cto.property_version_v1.clone(),
                                        cto.owner_address.clone(),
                                        cto.storage_id.clone(),
                                    ),
                                    cto,
                                );
                            }
                        }
                    },
                    WriteSetChange::WriteResource(resource) => {
                        if let Some((collection, current_collection)) =
                            CollectionV2::get_v2_from_write_resource(
                                resource,
                                txn_version,
                                wsc_index,
                                txn_timestamp,
                                &token_v2_metadata_helper,
                            )
                            .unwrap()
                        {
                            collections_v2.push(collection);
                            current_collections_v2.insert(
                                current_collection.collection_id.clone(),
                                current_collection,
                            );
                        }
                        if let Some((token_data, current_token_data)) =
                            TokenDataV2::get_v2_from_write_resource(
                                resource,
                                txn_version,
                                wsc_index,
                                txn_timestamp,
                                &token_v2_metadata_helper,
                            )
                            .unwrap()
                        {
                            // Add NFT ownership
                            if let Some(inner) = TokenOwnershipV2::get_nft_v2_from_token_data(
                                &token_data,
                                &token_v2_metadata_helper,
                            )
                            .unwrap()
                            {
                                let (
                                    nft_ownership,
                                    current_nft_ownership,
                                    from_nft_ownership,
                                    from_current_nft_ownership,
                                ) = inner;
                                token_ownerships_v2.push(nft_ownership);
                                // this is used to persist latest owner for burn event handling
                                prior_nft_ownership.insert(
                                    current_nft_ownership.token_data_id.clone(),
                                    NFTOwnershipV2 {
                                        token_data_id: current_nft_ownership.token_data_id.clone(),
                                        owner_address: current_nft_ownership.owner_address.clone(),
                                        is_soulbound: current_nft_ownership.is_soulbound_v2,
                                    },
                                );
                                current_token_ownerships_v2.insert(
                                    (
                                        current_nft_ownership.token_data_id.clone(),
                                        current_nft_ownership.property_version_v1.clone(),
                                        current_nft_ownership.owner_address.clone(),
                                        current_nft_ownership.storage_id.clone(),
                                    ),
                                    current_nft_ownership,
                                );
                                // Add the previous owner of the token transfer
                                if let Some(from_nft_ownership) = from_nft_ownership {
                                    let from_current_nft_ownership =
                                        from_current_nft_ownership.unwrap();
                                    token_ownerships_v2.push(from_nft_ownership);
                                    current_token_ownerships_v2.insert(
                                        (
                                            from_current_nft_ownership.token_data_id.clone(),
                                            from_current_nft_ownership.property_version_v1.clone(),
                                            from_current_nft_ownership.owner_address.clone(),
                                            from_current_nft_ownership.storage_id.clone(),
                                        ),
                                        from_current_nft_ownership,
                                    );
                                }
                            }
                            token_datas_v2.push(token_data);
                            current_token_datas_v2.insert(
                                current_token_data.token_data_id.clone(),
                                current_token_data,
                            );
                        }

                        // Add burned NFT handling
                        if let Some((nft_ownership, current_nft_ownership)) =
                            TokenOwnershipV2::get_burned_nft_v2_from_write_resource(
                                resource,
                                txn_version,
                                wsc_index,
                                txn_timestamp,
                                &tokens_burned,
                            )
                            .unwrap()
                        {
                            token_ownerships_v2.push(nft_ownership);
                            prior_nft_ownership.insert(
                                current_nft_ownership.token_data_id.clone(),
                                NFTOwnershipV2 {
                                    token_data_id: current_nft_ownership.token_data_id.clone(),
                                    owner_address: current_nft_ownership.owner_address.clone(),
                                    is_soulbound: current_nft_ownership.is_soulbound_v2,
                                },
                            );
                            current_token_ownerships_v2.insert(
                                (
                                    current_nft_ownership.token_data_id.clone(),
                                    current_nft_ownership.property_version_v1.clone(),
                                    current_nft_ownership.owner_address.clone(),
                                    current_nft_ownership.storage_id.clone(),
                                ),
                                current_nft_ownership,
                            );
                        }

                        // Add fungible token handling
                        if let Some((ft_ownership, current_ft_ownership)) =
                            TokenOwnershipV2::get_ft_v2_from_write_resource(
                                resource,
                                txn_version,
                                wsc_index,
                                txn_timestamp,
                                &token_v2_metadata_helper,
                            )
                            .unwrap()
                        {
                            token_ownerships_v2.push(ft_ownership);
                            current_token_ownerships_v2.insert(
                                (
                                    current_ft_ownership.token_data_id.clone(),
                                    current_ft_ownership.property_version_v1.clone(),
                                    current_ft_ownership.owner_address.clone(),
                                    current_ft_ownership.storage_id.clone(),
                                ),
                                current_ft_ownership,
                            );
                        }

                        // Track token properties
                        if let Some(token_metadata) = CurrentTokenV2Metadata::from_write_resource(
                            resource,
                            txn_version,
                            &token_v2_metadata_helper,
                        )
                        .unwrap()
                        {
                            current_token_v2_metadata.insert(
                                (
                                    token_metadata.object_address.clone(),
                                    token_metadata.resource_type.clone(),
                                ),
                                token_metadata,
                            );
                        }
                    },
                    WriteSetChange::DeleteResource(resource) => {
                        // Add burned NFT handling
                        if let Some((nft_ownership, current_nft_ownership)) =
                            TokenOwnershipV2::get_burned_nft_v2_from_delete_resource(
                                resource,
                                txn_version,
                                wsc_index,
                                txn_timestamp,
                                &prior_nft_ownership,
                                &tokens_burned,
                                conn,
                            )
                            .unwrap()
                        {
                            token_ownerships_v2.push(nft_ownership);
                            prior_nft_ownership.insert(
                                current_nft_ownership.token_data_id.clone(),
                                NFTOwnershipV2 {
                                    token_data_id: current_nft_ownership.token_data_id.clone(),
                                    owner_address: current_nft_ownership.owner_address.clone(),
                                    is_soulbound: current_nft_ownership.is_soulbound_v2,
                                },
                            );
                            current_token_ownerships_v2.insert(
                                (
                                    current_nft_ownership.token_data_id.clone(),
                                    current_nft_ownership.property_version_v1.clone(),
                                    current_nft_ownership.owner_address.clone(),
                                    current_nft_ownership.storage_id.clone(),
                                ),
                                current_nft_ownership,
                            );
                        }
                    },
                    _ => {},
                }
            }
        }
    }

    // Getting list of values and sorting by pk in order to avoid postgres deadlock since we're doing multi threaded db writes
    let mut current_collections_v2 = current_collections_v2
        .into_values()
        .collect::<Vec<CurrentCollectionV2>>();
    let mut current_token_datas_v2 = current_token_datas_v2
        .into_values()
        .collect::<Vec<CurrentTokenDataV2>>();
    let mut current_token_ownerships_v2 = current_token_ownerships_v2
        .into_values()
        .collect::<Vec<CurrentTokenOwnershipV2>>();
    let mut current_token_v2_metadata = current_token_v2_metadata
        .into_values()
        .collect::<Vec<CurrentTokenV2Metadata>>();

    // Sort by PK
    current_collections_v2.sort_by(|a, b| a.collection_id.cmp(&b.collection_id));
    current_token_datas_v2.sort_by(|a, b| a.token_data_id.cmp(&b.token_data_id));
    current_token_ownerships_v2.sort_by(|a, b| {
        (
            &a.token_data_id,
            &a.property_version_v1,
            &a.owner_address,
            &a.storage_id,
        )
            .cmp(&(
                &b.token_data_id,
                &b.property_version_v1,
                &b.owner_address,
                &b.storage_id,
            ))
    });
    current_token_v2_metadata.sort_by(|a, b| {
        (&a.object_address, &a.resource_type).cmp(&(&b.object_address, &b.resource_type))
    });

    (
        collections_v2,
        token_datas_v2,
        token_ownerships_v2,
        current_collections_v2,
        current_token_datas_v2,
        current_token_ownerships_v2,
        token_activities_v2,
        current_token_v2_metadata,
    )
}
