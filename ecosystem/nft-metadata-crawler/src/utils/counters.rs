// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_int_counter, register_int_counter_vec, IntCounter, IntCounterVec,
};
use once_cell::sync::Lazy;

// OVERALL METRICS

/// Number of times the NFT Metadata Crawler Parser has been invoked
pub static PARSER_INVOCATIONS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "nft_metadata_crawler_parser_invocation_count",
        "Number of times the parser has been invoked",
    )
    .unwrap()
});

/// Number of times the NFT Metadata Crawler Parser has completed successfully
pub static PARSER_SUCCESSES_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "nft_metadata_crawler_parser_success_count",
        "Number of times the parser has completed successfully",
    )
    .unwrap()
});

/// Number of times the NFT Metadata Crawler Parser has failed
pub static PARSER_FAIL_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "nft_metadata_crawler_parser_fail_count",
        "Number of times the parser has failed",
    )
    .unwrap()
});

/// Number of times the NFT Metadata Crawler Parser has received a URI marked as not to parse
pub static DO_NOT_PARSE_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "nft_metadata_crawler_parser_do_not_parse_count",
        "Number of times the parser received a URI marked as not to parse",
    )
    .unwrap()
});

// PUBSUB METRICS

/// Number of times a PubSub message has successfully been ACK'd
pub static PUBSUB_ACK_SUCCESS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "nft_metadata_crawler_parser_pubsub_ack_success_count",
        "Number of times a PubSub message has successfully been ACK'd",
    )
    .unwrap()
});

// POSTGRES METRICS

/// Number of times the connection pool has timed out when trying to get a connection
pub static UNABLE_TO_GET_CONNECTION_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_connection_pool_err",
        "Number of times the connection pool has timed out when trying to get a connection"
    )
    .unwrap()
});

/// Number of times the connection pool got a connection
pub static GOT_CONNECTION_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_connection_pool_ok",
        "Number of times the connection pool got a connection"
    )
    .unwrap()
});

// DEDUPLICATION METRICS

/// Number of times the NFT Metadata Crawler Parser has found a duplicate asset URI
pub static DUPLICATE_ASSET_URI_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "nft_metadata_crawler_parser_duplicate_asset_uri_count",
        "Number of times the NFT Metadata Crawler Parser has found a duplicate asset URI"
    )
    .unwrap()
});

/// Number of times the NFT Metadata Crawler Parser has found a duplicate raw image URI
pub static DUPLICATE_RAW_IMAGE_URI_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "nft_metadata_crawler_parser_duplicate_raw_image_uri_count",
        "Number of times the NFT Metadata Crawler Parser has found a duplicate raw image URI"
    )
    .unwrap()
});

/// Number of times the NFT Metadata Crawler Parser has found a duplicate raw animation URI
pub static DUPLICATE_RAW_ANIMATION_URI_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "nft_metadata_crawler_parser_duplicate_raw_animation_uri_count",
        "Number of times the NFT Metadata Crawler Parser has found a duplicate raw animation URI"
    )
    .unwrap()
});

// URI PARSER METRICS

/// Number of URIs skipped because of matches on the URI skip list
pub static SKIP_URI_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "nft_metadata_crawler_parser_skip_uri_count",
        "Number of URIs skipped because of matches on the URI skip list",
        &["reason"]
    )
    .unwrap()
});

/// Number of times the NFT Metadata Crawler Parser has invocated the URI Parser
pub static PARSE_URI_INVOCATION_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "nft_metadata_crawler_parser_parse_uri_invocation_count",
        "Number of times the NFT Metadata Crawler Parser has invocated the URI Parser"
    )
    .unwrap()
});

/// Number of times a given URI type has been parsed
pub static PARSE_URI_TYPE_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "nft_metadata_crawler_parser_parse_uri_type_count",
        "Number of times a given URI type has been parsed",
        &["uri_type"]
    )
    .unwrap()
});

// JSON PARSER METRICS

/// Number of times the NFT Metadata Crawler has invocated the JSON Parser
pub static PARSE_JSON_INVOCATION_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "nft_metadata_crawler_parser_parse_json_invocation_count",
        "Number of times the NFT Metadata Crawler Parser has invocated the JSON Parser"
    )
    .unwrap()
});

/// Number of times the NFT Metadata Crawler Parser has been able to parse a JSON
pub static SUCCESSFULLY_PARSED_JSON_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "nft_metadata_crawler_parser_successfully_parsed_json_count",
        "Number of times the NFT Metadata Crawler Parser has been able to parse a JSON"
    )
    .unwrap()
});

/// Number of times the NFT Metadata Crawler Parser has failed to parse a JSON and the error type
pub static FAILED_TO_PARSE_JSON_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "nft_metadata_crawler_parser_failed_to_parse_json_count",
        "Number of times the NFT Metadata Crawler Parser has failed to parse a JSON",
        &["error_type"]
    )
    .unwrap()
});

// IMAGE OPTIMIZER METRICS

/// Number of times the NFT Metadata Crawler Parser has invocated the Image Optimizer for an image
pub static OPTIMIZE_IMAGE_INVOCATION_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "nft_metadata_crawler_parser_optimize_image_invocation_count",
        "Number of times the NFT Metadata Crawler Parser has invocated the Image Optimizer for an image"
    )
    .unwrap()
});

/// Number of times a given image type has been optimized
pub static OPTIMIZE_IMAGE_TYPE_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "nft_metadata_crawler_parser_optimize_image_type_count",
        "Number of times a given image type has been optimized",
        &["image_type"]
    )
    .unwrap()
});

/// Number of times the Image Optimizer has been able to optimize an image
pub static SUCCESSFULLY_OPTIMIZED_IMAGE_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "nft_metadata_crawler_parser_successfully_optimized_image_count",
        "Number of times the Image Optimizer has been able to optimize an image"
    )
    .unwrap()
});

/// Number of times the Image Optimizer has failed to optimize an image and the error type
pub static FAILED_TO_OPTIMIZE_IMAGE_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "nft_metadata_crawler_parser_failed_to_optimize_image_count",
        "Number of times the NFT Metadata Crawler Parser has failed to optimize an image and the error type",
        &["error_type"]
    )
    .unwrap()
});

// GCS METRICS

/// Number of times the NFT Metadata Crawler Parser has attempted to upload to GCS
pub static GCS_UPLOAD_INVOCATION_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "nft_metadata_crawler_parser_gcs_upload_invocation_count",
        "Number of times the NFT Metadata Crawler Parser has attempted to upload to GCS"
    )
    .unwrap()
});

/// Number of times the NFT Metadata Crawler Parser has successfully uploaded to GCS
pub static SUCCESSFULLY_UPLOADED_TO_GCS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "nft_metadata_crawler_parser_successfully_uploaded_to_gcs_count",
        "Number of times the NFT Metadata Crawler Parser has successfully uploaded to GCS"
    )
    .unwrap()
});

/// Number of times the NFT Metadata Crawler Parser has failed to upload to GCS
pub static FAILED_TO_UPLOAD_TO_GCS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "nft_metadata_crawler_parser_failed_to_upload_to_gcs_count",
        "Number of times the NFT Metadata Crawler Parser has failed to upload to GCS"
    )
    .unwrap()
});
