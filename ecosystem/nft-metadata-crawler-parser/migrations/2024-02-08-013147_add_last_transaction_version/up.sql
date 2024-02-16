ALTER TABLE IF EXISTS nft_metadata_crawler.parsed_asset_uris ADD COLUMN last_transaction_version BIGINT NOT NULL DEFAULT 0;
