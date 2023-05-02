-- Your SQL goes here
CREATE TABLE IF NOT EXISTS delegated_staking_pool_balances (
  transaction_version BIGINT NOT NULL,
  staking_pool_address VARCHAR(66) NOT NULL,
  total_coins NUMERIC NOT NULL,
  total_shares NUMERIC NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (transaction_version, staking_pool_address)
);
CREATE INDEX dspb_insat_index ON delegated_staking_pool_balances (inserted_at);
CREATE TABLE IF NOT EXISTS current_delegated_staking_pool_balances (
  staking_pool_address VARCHAR(66) UNIQUE PRIMARY KEY NOT NULL,
  total_coins NUMERIC NOT NULL,
  total_shares NUMERIC NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX cdspb_insat_index ON current_delegated_staking_pool_balances (inserted_at);
ALTER TABLE current_delegator_balances
ADD COLUMN IF NOT EXISTS shares NUMERIC NOT NULL;
-- need this for delegation staking, changing to shares
CREATE OR REPLACE VIEW num_active_delegator_per_pool AS
SELECT pool_address,
  COUNT(DISTINCT delegator_address) AS num_active_delegator
FROM current_delegator_balances
WHERE shares > 0
GROUP BY 1;
ALTER TABLE current_delegator_balances DROP COLUMN IF EXISTS amount;