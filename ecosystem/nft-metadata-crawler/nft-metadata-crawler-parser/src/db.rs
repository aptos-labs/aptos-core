// Copyright © Aptos Foundation

use crate::{models::NFTMetadataCrawlerURIs, schema};
use diesel::{
    r2d2::{ConnectionManager, PooledConnection},
    upsert::excluded,
    ExpressionMethods, PgConnection, RunQueryDsl, SelectableHelper,
};
use std::error::Error;

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
                raw_animation_uri.eq(excluded(raw_animation_uri)),
                cdn_json_uri.eq(excluded(cdn_json_uri)),
                cdn_image_uri.eq(excluded(cdn_image_uri)),
                cdn_animation_uri.eq(excluded(cdn_animation_uri)),
                image_resizer_retry_count.eq(excluded(image_resizer_retry_count)),
                json_parser_retry_count.eq(excluded(json_parser_retry_count)),
                last_updated.eq(excluded(last_updated)),
            ))
            .returning(NFTMetadataCrawlerURIs::as_returning())
            .get_result(conn)?,
    )
}
