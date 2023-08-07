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
        json_parser_retry_count -> Int4,
        image_optimizer_retry_count -> Int4,
        animation_optimizer_retry_count -> Int4,
        inserted_at -> Timestamp,
    }
}
