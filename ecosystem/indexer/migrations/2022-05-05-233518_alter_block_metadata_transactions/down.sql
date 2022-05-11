-- This file should undo anything in `up.sql`
ALTER TABLE IF EXISTS block_metadata_transactions
    DROP COLUMN IF EXISTS epoch,
    DROP COLUMN IF EXISTS previous_block_votes_bitmap;
