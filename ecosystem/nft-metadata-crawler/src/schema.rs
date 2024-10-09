// @generated automatically by Diesel CLI.

pub mod nft_metadata_crawler {
    diesel::table! {
        nft_metadata_crawler.asset_uploader_request_statuses (request_id, asset_uri) {
            request_id -> Uuid,
            asset_uri -> Varchar,
            application_id -> Uuid,
            status_code -> Int8,
            error_message -> Nullable<Varchar>,
            cdn_image_uri -> Nullable<Varchar>,
            num_failures -> Int8,
            request_received_at -> Timestamp,
            inserted_at -> Timestamp,
        }
    }

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
            do_not_parse -> Bool,
            last_transaction_version -> Int8,
        }
    }

    diesel::allow_tables_to_appear_in_same_query!(
        asset_uploader_request_statuses,
        ledger_infos,
        parsed_asset_uris,
    );
}
