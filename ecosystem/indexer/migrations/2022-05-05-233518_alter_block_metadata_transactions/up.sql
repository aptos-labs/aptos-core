-- Your SQL goes here
ALTER TABLE block_metadata_transactions
    ADD COLUMN epoch BIGINT NOT NULL,
    ADD COLUMN previous_block_votes_bitmap jsonb NOT NULL;
