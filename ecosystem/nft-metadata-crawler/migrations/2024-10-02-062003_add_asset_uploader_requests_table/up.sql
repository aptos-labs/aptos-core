CREATE TABLE nft_metadata_crawler.asset_uploader_request_statuses (
  idempotency_key VARCHAR NOT NULL,
  application_id VARCHAR NOT NULL,
  asset_uri VARCHAR NOT NULL,
  status_code BIGINT NOT NULL DEFAULT 202,
  error_messages TEXT[],
  cdn_image_uri VARCHAR,
  num_failures BIGINT NOT NULL DEFAULT 0,
  request_received_at TIMESTAMP NOT NULL DEFAULT NOW(),
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  PRIMARY KEY (idempotency_key, application_id, asset_uri)
);
CREATE INDEX IF NOT EXISTS asset_uploader_status_code_inserted_at ON nft_metadata_crawler.asset_uploader_request_statuses (status_code, inserted_at);
