use std::fmt::Debug;

use field_count::FieldCount;

use crate::{
    database::{execute_with_better_error, get_chunks, PgDbPool, PgPoolConnection},
    models::marketplace_models::{
        bids::MarketplaceBids, collections::MarketplaceCollection, offers::MarketplaceOffer,
        orders::MarketplaceOrder,
    },
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

fn insert_offers(
    conn: &mut PgPoolConnection,
    offers: &[MarketplaceOffer],
) -> Result<(), diesel::result::Error> {
    let chunks = get_chunks(offers.len(), MarketplaceOffer::field_count());
    for (start_index, end_index) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::marketplace_offers::table)
                .values(&offers[start_index..end_index]),
            None,
        )?;
    }
    Ok(())
}

fn insert_orders(
    conn: &mut PgPoolConnection,
    orders: &[MarketplaceOrder],
) -> Result<(), diesel::result::Error> {
    let chunks = get_chunks(orders.len(), MarketplaceOffer::field_count());
    for (start_index, end_index) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::marketplace_orders::table)
                .values(&orders[start_index..end_index]),
            None,
        )?;
    }
    Ok(())
}

fn insert_bids(
    conn: &mut PgPoolConnection,
    bids: &[MarketplaceBids],
) -> Result<(), diesel::result::Error> {
    let chunks = get_chunks(bids.len(), MarketplaceOffer::field_count());
    for (start_index, end_index) in chunks {
        execute_with_better_error(
            conn,
            diesel::insert_into(schema::marketplace_bids::table)
                .values(&bids[start_index..end_index]),
            None,
        )?;
    }
    Ok(())
}
