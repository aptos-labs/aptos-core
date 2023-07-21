// Copyright © Aptos Foundation

use crate::schema::nft_metadata_crawler_uris;
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
use serde::{Deserialize, Serialize};
use std::{thread, time::Duration};

#[derive(Debug, Deserialize, Identifiable, Queryable, Serialize)]
#[diesel(primary_key(token_uri))]
#[diesel(table_name = nft_metadata_crawler_uris)]
pub struct NFTMetadataCrawlerURIsQuery {
    pub token_uri: String,
    pub raw_image_uri: Option<String>,
    pub raw_animation_uri: Option<String>,
    pub cdn_json_uri: Option<String>,
    pub cdn_image_uri: Option<String>,
    pub cdn_animation_uri: Option<String>,
    pub json_parser_retry_count: i32,
    pub image_optimizer_retry_count: i32,
    pub animation_optimizer_retry_count: i32,
    pub inserted_at: chrono::NaiveDateTime,
}

impl NFTMetadataCrawlerURIsQuery {
    pub fn get_by_token_uri(
        token_uri: String,
        conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    ) -> diesel::QueryResult<Option<Self>> {
        for _ in 0..3 {
            match nft_metadata_crawler_uris::table
                .find(token_uri.clone())
                .first::<NFTMetadataCrawlerURIsQuery>(conn)
                .optional()
            {
                Ok(result) => return Ok(result),
                Err(_) => thread::sleep(Duration::from_secs(5)),
            }
        }

        // Retry one last time but now propagate the error if it fails
        nft_metadata_crawler_uris::table
            .find(token_uri)
            .first::<NFTMetadataCrawlerURIsQuery>(conn)
            .optional()
    }

    pub fn get_by_raw_image_uri(
        raw_image_uri: Option<String>,
        conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    ) -> diesel::QueryResult<Option<Self>> {
        if raw_image_uri.is_none() {
            return Ok(None);
        }

        for _ in 0..3 {
            match nft_metadata_crawler_uris::table
                .filter(nft_metadata_crawler_uris::raw_image_uri.eq(raw_image_uri.clone()))
                .first::<NFTMetadataCrawlerURIsQuery>(conn)
                .optional()
            {
                Ok(result) => return Ok(result),
                Err(_) => thread::sleep(Duration::from_secs(5)),
            }
        }

        // Retry one last time but now propagate the error if it fails
        nft_metadata_crawler_uris::table
            .filter(nft_metadata_crawler_uris::raw_image_uri.eq(raw_image_uri))
            .first::<NFTMetadataCrawlerURIsQuery>(conn)
            .optional()
    }

    pub fn get_by_raw_animation_uri(
        raw_animation_uri: Option<String>,
        conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    ) -> diesel::QueryResult<Option<Self>> {
        if raw_animation_uri.is_none() {
            return Ok(None);
        }

        for _ in 0..3 {
            match nft_metadata_crawler_uris::table
                .filter(nft_metadata_crawler_uris::raw_animation_uri.eq(raw_animation_uri.clone()))
                .first::<NFTMetadataCrawlerURIsQuery>(conn)
                .optional()
            {
                Ok(result) => return Ok(result),
                Err(_) => thread::sleep(Duration::from_secs(5)),
            }
        }

        // Retry one last time but now propagate the error if it fails
        nft_metadata_crawler_uris::table
            .filter(nft_metadata_crawler_uris::raw_animation_uri.eq(raw_animation_uri))
            .first::<NFTMetadataCrawlerURIsQuery>(conn)
            .optional()
    }
}
