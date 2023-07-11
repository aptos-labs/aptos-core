CREATE TABLE nft_metadata_crawler_entry (
  token_data_id VARCHAR UNIQUE PRIMARY KEY NOT NULL,
  token_uri VARCHAR NOT NULL,
  last_transaction_version INT NOT NULL,
  last_transaction_timestamp TIMESTAMP NOT NULL,
  last_updated TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE TABLE nft_metadata_crawler_uris (
  token_uri VARCHAR UNIQUE PRIMARY KEY NOT NULL,
  raw_image_uri VARCHAR,
  cdn_json_uri VARCHAR,
  cdn_image_uri VARCHAR,
  image_resizer_retry_count INT NOT NULL,
  json_parser_retry_count INT NOT NULL,
  last_updated TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX nft_token_uri ON nft_metadata_crawler_entry (token_uri);
