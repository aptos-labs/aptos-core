-- Your SQL goes here
ALTER TABLE block_metadata_transactions
ADD COLUMN failed_proposer_indices jsonb NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE block_metadata_transactions
ALTER column failed_proposer_indices DROP DEFAULT;
