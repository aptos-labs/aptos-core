// Copyright © Aptos Foundation

use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
use field_count::FieldCount;
use serde::{Deserialize, Serialize};

use crate::schema::nft_metadata_crawler_uris;

#[derive(Clone, Debug, Deserialize, FieldCount, Identifiable, Insertable, Serialize)]
#[diesel(primary_key(token_uri))]
#[diesel(table_name = nft_metadata_crawler_uris)]
pub struct NFTMetadataCrawlerURIs {
    pub token_uri: String,
    pub raw_image_uri: Option<String>,
    pub raw_animation_uri: Option<String>,
    pub cdn_json_uri: Option<String>,
    pub cdn_image_uri: Option<String>,
    pub cdn_animation_uri: Option<String>,
    pub image_optimizer_retry_count: i32,
    pub json_parser_retry_count: i32,
}

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
    pub image_optimizer_retry_count: i32,
    pub json_parser_retry_count: i32,
    pub last_updated: chrono::NaiveDateTime,
}

impl NFTMetadataCrawlerURIsQuery {
    pub fn get_by_token_uri(
        token_uri: String,
        conn: &mut PooledConnection<ConnectionManager<PgConnection>>,
    ) -> diesel::QueryResult<Option<Self>> {
        nft_metadata_crawler_uris::table
            .find(token_uri)
            .first::<NFTMetadataCrawlerURIsQuery>(conn)
            .optional()
    }
}
