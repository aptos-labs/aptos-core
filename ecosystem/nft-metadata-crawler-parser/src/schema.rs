// Copyright © Aptos Foundation

// @generated automatically by Diesel CLI.

pub mod nft_metadata_crawler {
    diesel::table! {
        nft_metadata_crawler.ledger_infos (chain_id) {
            chain_id -> Int8,
        }
    }

    diesel::table! {
        nft_metadata_crawler.parsed_asset_uris (asset_uri) {
            asset_uri -> Varchar,
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

    diesel::allow_tables_to_appear_in_same_query!(ledger_infos, parsed_asset_uris,);
}
