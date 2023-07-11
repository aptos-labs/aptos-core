// Copyright © Aptos Foundation

use crate::{
    models::{NFTMetadataCrawlerEntry, NFTMetadataCrawlerURIs},
    schema,
};
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    upsert::excluded,
    ExpressionMethods, PgConnection, RunQueryDsl, SelectableHelper,
};
use std::error::Error;

pub fn upsert_entry(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    entry: NFTMetadataCrawlerEntry,
) -> Result<NFTMetadataCrawlerEntry, Box<dyn Error + Send + Sync>> {
    use schema::nft_metadata_crawler_entry::dsl::*;

    Ok(
        diesel::insert_into(schema::nft_metadata_crawler_entry::table)
            .values(&entry)
            .on_conflict(token_data_id)
            .do_update()
            .set((
                token_uri.eq(excluded(token_uri)),
                last_transaction_version.eq(excluded(last_transaction_version)),
                last_transaction_timestamp.eq(excluded(last_transaction_timestamp)),
                last_updated.eq(excluded(last_updated)),
            ))
            .returning(NFTMetadataCrawlerEntry::as_returning())
            .get_result(conn)?,
    )
}

pub fn upsert_uris(
    conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    entry: NFTMetadataCrawlerURIs,
) -> Result<NFTMetadataCrawlerURIs, Box<dyn Error + Send + Sync>> {
    use schema::nft_metadata_crawler_uris::dsl::*;

    Ok(
        diesel::insert_into(schema::nft_metadata_crawler_uris::table)
            .values(&entry)
            .on_conflict(token_uri)
            .do_update()
            .set((
                raw_image_uri.eq(excluded(raw_image_uri)),
                cdn_json_uri.eq(excluded(cdn_json_uri)),
                cdn_image_uri.eq(excluded(cdn_image_uri)),
                image_resizer_retry_count.eq(excluded(image_resizer_retry_count)),
                json_parser_retry_count.eq(excluded(json_parser_retry_count)),
                last_updated.eq(excluded(last_updated)),
            ))
            .returning(NFTMetadataCrawlerURIs::as_returning())
            .get_result(conn)?,
    )
}
