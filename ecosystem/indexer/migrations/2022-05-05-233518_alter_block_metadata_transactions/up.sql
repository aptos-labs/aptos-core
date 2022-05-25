-- Your SQL goes here
ALTER TABLE block_metadata_transactions
ADD COLUMN epoch BIGINT NOT NULL DEFAULT -1,
ADD COLUMN previous_block_votes_bitmap jsonb NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE block_metadata_transactions
ALTER column epoch DROP DEFAULT,
ALTER column previous_block_votes_bitmap DROP DEFAULT;
