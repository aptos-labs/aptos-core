ALTER TABLE IF EXISTS nft_metadata_crawler.parsed_asset_uris ADD COLUMN IF NOT EXISTS do_not_parse BOOLEAN NOT NULL DEFAULT FALSE;
