-- Your SQL goes here
-- need this to query transactions that touch an account's events
CREATE OR REPLACE VIEW address_version_from_events AS
SELECT account_address,
  transaction_version
FROM events
GROUP BY 1,
  2;
-- need this to query transactions that touch an account's move resources
CREATE OR REPLACE VIEW address_version_from_move_resources AS
SELECT address,
  transaction_version
FROM move_resources
GROUP BY 1,
  2;
-- need this for getting NFTs grouped by collections
CREATE OR REPLACE VIEW current_collection_ownership_view AS
SELECT owner_address,
  creator_address,
  collection_name,
  collection_data_id_hash,
  MAX(last_transaction_version) AS last_transaction_version,
  COUNT(DISTINCT name) AS distinct_tokens
FROM current_token_ownerships
WHERE amount > 0
GROUP BY 1,
  2,
  3,
  4;
-- need this for delegation staking
CREATE OR REPLACE VIEW num_active_delegator_per_pool AS
SELECT pool_address,
  COUNT(DISTINCT delegator_address) AS num_active_delegator
FROM current_delegator_balances
WHERE amount > 0
GROUP BY 1;
-- indices
CREATE INDEX IF NOT EXISTS curr_to_collection_hash_owner_index ON current_token_ownerships (collection_data_id_hash, owner_address);