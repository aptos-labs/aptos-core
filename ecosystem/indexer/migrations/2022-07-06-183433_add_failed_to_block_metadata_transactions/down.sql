-- This file should undo anything in `up.sql`
ALTER TABLE IF EXISTS block_metadata_transactions
    DROP COLUMN IF EXISTS failed_proposer_indices;
