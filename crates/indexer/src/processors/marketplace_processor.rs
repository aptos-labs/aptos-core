use std::fmt::Debug;

use field_count::FieldCount;

use crate::{
    database::{execute_with_better_error, get_chunks, PgDbPool, PgPoolConnection},
    models::marketplace_models::collections::MarketplaceCollection,
    schema,
};

pub const NAME: &str = "marketplace_processor";

pub struct MarketplaceProcessor {
    connection_pool: PgDbPool,
}

impl MarketplaceProcessor {
    pub fn new(connection_pool: PgDbPool) -> Self {
        Self { connection_pool }
    }
}

impl Debug for MarketplaceProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let state = &self.connection_pool.state();
        write!(
            f,
            "MarketplaceProcessor {{ connections: {:?}  idle_connections: {:?} }}",
            state.connections, state.idle_connections
        )
    }
}

fn insert_collections(
    conn: &mut PgPoolConnection,
    collections: &[MarketplaceCollection],
) -> Result<(), diesel::result::Error> {
    let chunks = get_chunks(collections.len(), MarketplaceCollection::field_count());
    for (start_index, end_index) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::marketplace_collections::table)
                .values(&collections[start_index..end_index]),
            None,
        )?;
    }
    Ok(())
}
