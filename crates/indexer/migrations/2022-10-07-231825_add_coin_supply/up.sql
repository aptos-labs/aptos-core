-- Your SQL goes here
-- coin supply, currently velor coin only
CREATE TABLE coin_supply (
  transaction_version BIGINT NOT NULL,
  -- Hash of the non-truncated coin type
  coin_type_hash VARCHAR(64) NOT NULL,
  coin_type VARCHAR(5000) NOT NULL,
  supply NUMERIC NOT NULL,
  transaction_timestamp TIMESTAMP NOT NULL,
  transaction_epoch BIGINT NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (transaction_version, coin_type_hash)
);
CREATE INDEX cs_ct_tv_index on coin_supply (coin_type, transaction_version desc);
CREATE INDEX cs_epoch_index on coin_supply (transaction_epoch);
-- Add coin supply aggregator handle to coin infos to be able to access total supply data
ALTER TABLE coin_infos
ADD COLUMN supply_aggregator_table_handle VARCHAR(66),
  ADD COLUMN supply_aggregator_table_key TEXT;
-- Add description to token_datas and current_token_datas
ALTER TABLE token_datas
ADD COLUMN description TEXT NOT NULL;
ALTER TABLE current_token_datas
ADD COLUMN description TEXT NOT NULL;
-- Add epoch to user transactions and transactions
ALTER TABLE user_transactions
ADD COLUMN epoch BIGINT NOT NULL;
ALTER TABLE transactions
ADD COLUMN epoch BIGINT NOT NULL;
-- Create index on epoch for easy queries
CREATE INDEX ut_epoch_index ON user_transactions (epoch);
CREATE INDEX txn_epoch_index ON transactions (epoch);