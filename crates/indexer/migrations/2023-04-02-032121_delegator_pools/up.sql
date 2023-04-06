-- Your SQL goes here
CREATE TABLE IF NOT EXISTS delegated_staking_pools (
  staking_pool_address VARCHAR(66) UNIQUE PRIMARY KEY NOT NULL,
  first_transaction_version BIGINT NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX dsp_insat_index ON delegated_staking_pools (inserted_at);
ALTER TABLE current_staking_pool_voter
ADD COLUMN IF NOT EXISTS operator_address VARCHAR(66) NOT NULL;