-- Your SQL goes here
CREATE INDEX txn_v_index ON transactions (version desc);
CREATE INDEX ev_tv_index ON events (transaction_version desc);
CREATE INDEX wsc_v_index ON write_set_changes (transaction_version desc);
CREATE INDEX bmt_v_index ON block_metadata_transactions (version desc);
CREATE INDEX bmt_ts_index ON block_metadata_transactions ("timestamp" desc);
-- coin supply, currently aptos coin only
CREATE TABLE coin_supply (
  transaction_version BIGINT NOT NULL,
  -- Hash of the non-truncated coin type
  coin_type_hash VARCHAR(64) NOT NULL,
  coin_type VARCHAR(5000) NOT NULL,
  supply NUMERIC NOT NULL,
  transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (transaction_version, coin_type_hash)
);
CREATE INDEX cs_ct_tv_index on coin_supply (coin_type, transaction_version desc);
-- Add coin supply aggregator handle to coin infos to be able to access total supply data
ALTER TABLE coin_infos
ADD COLUMN supply_aggregator_table_handle VARCHAR(66),
  ADD COLUMN supply_aggregator_table_key TEXT;