CREATE TABLE IF NOT EXISTS nft_metadata_crawler_uris (
  token_uri VARCHAR UNIQUE PRIMARY KEY NOT NULL,
  raw_image_uri VARCHAR,
  raw_animation_uri VARCHAR,
  cdn_json_uri VARCHAR,
  cdn_image_uri VARCHAR,
  cdn_animation_uri VARCHAR,
  image_optimizer_retry_count INT NOT NULL,
  json_parser_retry_count INT NOT NULL,
  last_updated TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS nft_raw_image_uri ON nft_metadata_crawler_uris (raw_image_uri);
CREATE INDEX IF NOT EXISTS nft_raw_animation_uri ON nft_metadata_crawler_uris (raw_animation_uri);
