// Copyright © Aptos Foundation

use chrono::{NaiveDateTime, Utc};
use diesel::prelude::*;
use std::error::Error;

#[derive(Clone, Insertable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::nft_metadata_crawler_entry)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NFTMetadataCrawlerEntry {
    pub token_data_id: String,
    pub token_uri: String,
    pub last_transaction_version: i32,
    pub last_transaction_timestamp: chrono::NaiveDateTime,
    pub last_updated: chrono::NaiveDateTime,
}

impl NFTMetadataCrawlerEntry {
    pub fn new(s: String) -> Result<(Self, bool), Box<dyn Error + Send + Sync>> {
        let parts: Vec<&str> = s.split(',').collect();
        if parts.len() == 5 {
            Ok((
                Self {
                    token_data_id: parts[0].to_string(),
                    token_uri: parts[1].to_string(),
                    last_transaction_version: parts[2].to_string().parse()?,
                    last_transaction_timestamp: NaiveDateTime::parse_from_str(
                        parts[3],
                        "%Y-%m-%d %H:%M:%S %Z",
                    )
                    .unwrap_or(NaiveDateTime::parse_from_str(
                        parts[3],
                        "%Y-%m-%d %H:%M:%S%.f %Z",
                    )?),
                    last_updated: Utc::now().naive_utc(),
                },
                parts[4].parse::<bool>().unwrap_or(false),
            ))
        } else {
            Err("Error parsing record".into())
        }
    }
}

#[derive(Clone, Insertable, Queryable, Selectable)]
#[diesel(table_name = crate::schema::nft_metadata_crawler_uris)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NFTMetadataCrawlerURIs {
    pub token_uri: String,
    pub raw_image_uri: Option<String>,
    pub cdn_json_uri: Option<String>,
    pub cdn_image_uri: Option<String>,
    pub image_resizer_retry_count: i32,
    pub json_parser_retry_count: i32,
    pub last_updated: chrono::NaiveDateTime,
}
