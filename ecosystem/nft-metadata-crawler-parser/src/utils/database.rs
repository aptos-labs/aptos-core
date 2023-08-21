// Copyright © Aptos Foundation

use crate::{
    models::{ledger_info::LedgerInfo, nft_metadata_crawler_uris::NFTMetadataCrawlerURIs},
    schema,
};
use anyhow::Context;
use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    upsert::excluded,
    ExpressionMethods, PgConnection, RunQueryDsl,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tracing::{debug, info};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

/// Establishes a connection pool to Postgres
pub fn establish_connection_pool(database_url: String) -> Pool<ConnectionManager<PgConnection>> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder()
        .build(manager)
        .expect("Failed to create pool.")
}

/// Runs database migrations
pub fn run_migrations(pool: &Pool<ConnectionManager<PgConnection>>) {
    pool.get()
        .expect("[NFT Metadata Crawler] Could not get connection for migrations")
        .run_pending_migrations(MIGRATIONS)
        .expect("[NFT Metadata Crawler] migrations failed!");
}

/// Upserts URIs into database
pub fn upsert_uris(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    entry: NFTMetadataCrawlerURIs,
) -> anyhow::Result<usize> {
    use schema::nft_metadata_crawler::parsed_token_uris::dsl::*;

    let query = diesel::insert_into(schema::nft_metadata_crawler::parsed_token_uris::table)
        .values(&entry)
        .on_conflict(token_uri)
        .do_update()
        .set((
            raw_image_uri.eq(excluded(raw_image_uri)),
            raw_animation_uri.eq(excluded(raw_animation_uri)),
            cdn_json_uri.eq(excluded(cdn_json_uri)),
            cdn_image_uri.eq(excluded(cdn_image_uri)),
            cdn_animation_uri.eq(excluded(cdn_animation_uri)),
            image_optimizer_retry_count.eq(excluded(image_optimizer_retry_count)),
            json_parser_retry_count.eq(excluded(json_parser_retry_count)),
            animation_optimizer_retry_count.eq(excluded(animation_optimizer_retry_count)),
        ));

    let debug_query = diesel::debug_query::<diesel::pg::Pg, _>(&query).to_string();
    debug!("Executing Query: {}", debug_query);
    query.execute(conn).context(debug_query)
}

/// Verify the chain id from PubSub against the database.
pub fn check_or_update_chain_id(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    pubsub_chain_id: i64,
) -> anyhow::Result<u64> {
    info!("[NFT Metadata Crawler] Checking if chain id is correct");

    let maybe_existing_chain_id = LedgerInfo::get(conn)?.map(|li| li.chain_id);

    match maybe_existing_chain_id {
        Some(chain_id) => {
            anyhow::ensure!(chain_id == pubsub_chain_id, "[NFT Metadata Crawler] Wrong chain detected! Trying to index chain {} now but existing data is for chain {}", pubsub_chain_id, chain_id);
            info!(
                chain_id = chain_id,
                "[NFT Metadata Crawler] Chain id matches! Continue to index...",
            );
            Ok(chain_id as u64)
        },
        None => {
            info!(
                chain_id = pubsub_chain_id,
                "[NFT Metadata Crawler] Adding chain id to db, continue to index.."
            );
            insert_chain_id(conn, pubsub_chain_id).map(|_| pubsub_chain_id as u64)
        },
    }
}

/// Updates chain id in database
fn insert_chain_id(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    grpc_chain_id: i64,
) -> anyhow::Result<usize> {
    let query =
        diesel::insert_into(schema::nft_metadata_crawler::ledger_infos::table).values(LedgerInfo {
            chain_id: grpc_chain_id,
        });

    let debug_query = diesel::debug_query::<diesel::pg::Pg, _>(&query).to_string();
    debug!("Executing Query: {}", debug_query);
    query.execute(conn).context(debug_query)
}
