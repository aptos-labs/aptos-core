-- Your SQL goes here
-- get delegated staking events such as withdraw , unlock, add stake, etc. 
CREATE TABLE delegated_staking_activities (
  transaction_version BIGINT NOT NULL,
  event_index BIGINT NOT NULL,
  delegator_address VARCHAR(66) NOT NULL,
  pool_address VARCHAR(66) NOT NULL,
  event_type text NOT NULL,
  amount NUMERIC NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (transaction_version, event_index)
);
CREATE INDEX dsa_pa_da_index ON delegated_staking_activities (
  pool_address,
  delegator_address,
  transaction_version asc,
  event_index asc
);
CREATE INDEX dsa_insat_index ON delegated_staking_activities (inserted_at);
-- estimates how much delegator has staked in a pool (currently supports active only)
CREATE TABLE current_delegator_balances (
  delegator_address VARCHAR(66) NOT NULL,
  pool_address VARCHAR(66) NOT NULL,
  pool_type VARCHAR(100) NOT NULL,
  table_handle VARCHAR(66) NOT NULL,
  amount NUMERIC NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (delegator_address, pool_address, pool_type)
);
CREATE INDEX cdb_insat_index ON delegated_staking_activities (inserted_at);