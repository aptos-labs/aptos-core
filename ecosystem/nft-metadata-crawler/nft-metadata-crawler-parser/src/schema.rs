// Copyright © Aptos Foundation

// @generated automatically by Diesel CLI.

diesel::table! {
    nft_metadata_crawler_uris (token_uri) {
        token_uri -> Varchar,
        raw_image_uri -> Nullable<Varchar>,
        raw_animation_uri -> Nullable<Varchar>,
        cdn_json_uri -> Nullable<Varchar>,
        cdn_image_uri -> Nullable<Varchar>,
        cdn_animation_uri -> Nullable<Varchar>,
        image_resizer_retry_count -> Int4,
        json_parser_retry_count -> Int4,
        last_updated -> Timestamp,
    }
}
