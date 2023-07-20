// Copyright © Aptos Foundation

use crate::models::nft_metadata_crawler_uris::NFTMetadataCrawlerURIs;
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};
use nft_metadata_crawler_utils::NFTMetadataCrawlerEntry;

/**
 * Stuct that represents a parser for a single entry from queue
 */
#[allow(dead_code)]
pub struct Parser {
    entry: NFTMetadataCrawlerEntry,
    model: NFTMetadataCrawlerURIs,
    bucket: String,
    token: String,
    conn: PooledConnection<ConnectionManager<PgConnection>>,
    cdn_prefix: String,
}

impl Parser {
    pub fn new(
        entry: NFTMetadataCrawlerEntry,
        bucket: String,
        token: String,
        conn: PooledConnection<ConnectionManager<PgConnection>>,
        cdn_prefix: String,
    ) -> Self {
        Self {
            model: NFTMetadataCrawlerURIs::new(entry.token_uri.clone()),
            entry,
            bucket,
            token,
            conn,
            cdn_prefix,
        }
    }

    /**
     * Main parsing flow
     */
    pub async fn parse(&mut self) -> anyhow::Result<()> {
        todo!();
    }
}
