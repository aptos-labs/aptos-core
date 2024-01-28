// Copyright © Aptos Foundation

use crate::{models::ledger_info::LedgerInfo, schema};
use anyhow::Context;
use diesel::{
    r2d2::{ConnectionManager, Pool, PooledConnection},
    PgConnection, RunQueryDsl,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use tracing::{debug, info};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

/// Establishes a connection pool to Postgres
pub fn establish_connection_pool(database_url: &String) -> Pool<ConnectionManager<PgConnection>> {
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder()
        .build(manager)
        .expect("Failed to create pool.")
}

/// Runs database migrations
pub fn run_migrations(pool: &Pool<ConnectionManager<PgConnection>>) {
    pool.get()
        .expect("[Event Stream] Could not get connection for migrations")
        .run_pending_migrations(MIGRATIONS)
        .expect("[Event Stream] migrations failed!");
}

/// Verify the chain id from PubSub against the database.
pub fn check_or_update_chain_id(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    pubsub_chain_id: i64,
) -> anyhow::Result<u64> {
    info!("[Event Stream] Checking if chain id is correct");

    let maybe_existing_chain_id = LedgerInfo::get(conn)?.map(|li| li.chain_id);

    match maybe_existing_chain_id {
        Some(chain_id) => {
            anyhow::ensure!(chain_id == pubsub_chain_id, "[Event Stream] Wrong chain detected! Trying to index chain {} now but existing data is for chain {}", pubsub_chain_id, chain_id);
            info!(
                chain_id = chain_id,
                "[Event Stream] Chain id matches! Continue to stream...",
            );
            Ok(chain_id as u64)
        },
        None => {
            info!(
                chain_id = pubsub_chain_id,
                "[Event Stream] Adding chain id to db, continue to stream.."
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
    let query = diesel::insert_into(schema::event_stream::ledger_infos::table).values(LedgerInfo {
        chain_id: grpc_chain_id,
    });

    let debug_query = diesel::debug_query::<diesel::pg::Pg, _>(&query).to_string();
    debug!("Executing Query: {}", debug_query);
    query.execute(conn).context(debug_query)
}
