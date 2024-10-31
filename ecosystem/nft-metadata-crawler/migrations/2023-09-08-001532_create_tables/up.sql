CREATE SCHEMA IF NOT EXISTS nft_metadata_crawler;

CREATE TABLE IF NOT EXISTS nft_metadata_crawler.parsed_asset_uris (
  asset_uri VARCHAR UNIQUE PRIMARY KEY NOT NULL,
  raw_image_uri VARCHAR,
  raw_animation_uri VARCHAR,
  cdn_json_uri VARCHAR,
  cdn_image_uri VARCHAR,
  cdn_animation_uri VARCHAR,
  json_parser_retry_count INT NOT NULL,
  image_optimizer_retry_count INT NOT NULL,
  animation_optimizer_retry_count INT NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS nft_metadata_crawler.ledger_infos (chain_id BIGINT UNIQUE PRIMARY KEY NOT NULL);

CREATE INDEX IF NOT EXISTS nft_raw_image_uri ON nft_metadata_crawler.parsed_asset_uris (raw_image_uri);
CREATE INDEX IF NOT EXISTS nft_raw_animation_uri ON nft_metadata_crawler.parsed_asset_uris (raw_animation_uri);
CREATE INDEX IF NOT EXISTS nft_inserted_at ON nft_metadata_crawler.parsed_asset_uris (inserted_at);
