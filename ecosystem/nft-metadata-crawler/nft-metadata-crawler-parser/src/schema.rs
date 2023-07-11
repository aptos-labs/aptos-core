// Copyright © Aptos Foundation

// @generated automatically by Diesel CLI.

diesel::table! {
    nft_metadata_crawler_entry (token_data_id) {
        token_data_id -> Varchar,
        token_uri -> Varchar,
        last_transaction_version -> Int4,
        last_transaction_timestamp -> Timestamp,
        last_updated -> Timestamp,
    }
}

diesel::table! {
    nft_metadata_crawler_uris (token_uri) {
        token_uri -> Varchar,
        raw_image_uri -> Nullable<Varchar>,
        cdn_json_uri -> Nullable<Varchar>,
        cdn_image_uri -> Nullable<Varchar>,
        image_resizer_retry_count -> Int4,
        json_parser_retry_count -> Int4,
        last_updated -> Timestamp,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    nft_metadata_crawler_entry,
    nft_metadata_crawler_uris,
);
