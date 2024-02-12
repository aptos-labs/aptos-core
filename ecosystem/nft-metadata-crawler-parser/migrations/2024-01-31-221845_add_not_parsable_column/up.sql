ALTER TABLE IF EXISTS nft_metadata_crawler.parsed_asset_uris ADD COLUMN do_not_parse BOOLEAN NOT NULL DEFAULT FALSE;
