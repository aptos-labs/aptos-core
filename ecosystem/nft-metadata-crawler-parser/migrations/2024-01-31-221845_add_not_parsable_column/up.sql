ALTER TABLE IF NOT EXISTS nft_metadata_crawler.parsed_asset_uris ADD COLUMN do_not_parse BOOLEAN NOT NULL DEFAULT FALSE;
