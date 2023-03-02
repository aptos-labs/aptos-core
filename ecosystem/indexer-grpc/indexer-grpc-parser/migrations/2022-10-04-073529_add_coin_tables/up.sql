-- Your SQL goes here
CREATE VIEW move_resources_view AS
SELECT transaction_version,
  write_set_change_index,
  transaction_block_height,
  name,
  address,
  "type",
  "module",
  generic_type_params,
  data#>>'{}' as json_data,
  is_deleted,
  inserted_at
FROM move_resources;
-- adding timestamp to all token tables
ALTER TABLE token_activities
ADD COLUMN transaction_timestamp TIMESTAMP NOT NULL;
ALTER TABLE current_token_pending_claims
ADD COLUMN last_transaction_timestamp TIMESTAMP NOT NULL;
ALTER TABLE current_token_ownerships
ADD COLUMN last_transaction_timestamp TIMESTAMP NOT NULL;
ALTER TABLE current_token_datas
ADD COLUMN last_transaction_timestamp TIMESTAMP NOT NULL;
ALTER TABLE current_collection_datas
ADD COLUMN last_transaction_timestamp TIMESTAMP NOT NULL;
ALTER TABLE tokens
ADD COLUMN transaction_timestamp TIMESTAMP NOT NULL;
ALTER TABLE token_ownerships
ADD COLUMN transaction_timestamp TIMESTAMP NOT NULL;
ALTER TABLE token_datas
ADD COLUMN transaction_timestamp TIMESTAMP NOT NULL;
ALTER TABLE collection_datas
ADD COLUMN transaction_timestamp TIMESTAMP NOT NULL;
-- coin infos. Only first transaction matters
CREATE TABLE coin_infos (
  -- Hash of the non-truncated coin type
  coin_type_hash VARCHAR(64) UNIQUE PRIMARY KEY NOT NULL,
  -- creator_address::name::symbol<struct>
  coin_type VARCHAR(5000) NOT NULL,
  -- transaction version where coin info was first defined
  transaction_version_created BIGINT NOT NULL,
  creator_address VARCHAR(66) NOT NULL,
  name VARCHAR(32) NOT NULL,
  symbol VARCHAR(10) NOT NULL,
  decimals INT NOT NULL,
  transaction_created_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX ci_ct_index on coin_infos (coin_type);
CREATE INDEX ci_ca_name_symbol_index on coin_infos (creator_address, name, symbol);
CREATE INDEX ci_insat_index ON coin_infos (inserted_at);
-- current coin owned by user
CREATE TABLE coin_balances (
  transaction_version BIGINT NOT NULL,
  owner_address VARCHAR(66) NOT NULL,
  -- Hash of the non-truncated coin type
  coin_type_hash VARCHAR(64) NOT NULL,
  -- creator_address::name::symbol<struct>
  coin_type VARCHAR(5000) NOT NULL,
  amount NUMERIC NOT NULL,
  transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (
    transaction_version,
    owner_address,
    coin_type_hash
  )
);
CREATE INDEX cb_tv_oa_ct_index on coin_balances (transaction_version, owner_address, coin_type);
CREATE INDEX cb_oa_ct_index on coin_balances (owner_address, coin_type);
CREATE INDEX cb_ct_a_index on coin_balances (coin_type, amount);
CREATE INDEX cb_insat_index ON coin_balances (inserted_at);
-- current coin owned by user
CREATE TABLE current_coin_balances (
  owner_address VARCHAR(66) NOT NULL,
  -- Hash of the non-truncated coin type
  coin_type_hash VARCHAR(64) NOT NULL,
  -- creator_address::name::symbol<struct>
  coin_type VARCHAR(5000) NOT NULL,
  amount NUMERIC NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  last_transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (owner_address, coin_type_hash)
);
CREATE INDEX ccb_oa_ct_index on current_coin_balances (owner_address, coin_type);
CREATE INDEX ccb_ct_a_index on current_coin_balances (coin_type, amount);
CREATE INDEX ccb_insat_index on current_coin_balances (inserted_at);
-- coinstore activities (send, receive, gas fees). Mint/burn not supported because event missing
CREATE TABLE coin_activities (
  transaction_version BIGINT NOT NULL,
  event_account_address VARCHAR(66) NOT NULL,
  event_creation_number BIGINT NOT NULL,
  event_sequence_number BIGINT NOT NULL,
  owner_address VARCHAR(66) NOT NULL,
  -- creator_address::name::symbol
  coin_type VARCHAR(5000) NOT NULL,
  amount NUMERIC NOT NULL,
  activity_type VARCHAR(200) NOT NULL,
  is_gas_fee BOOLEAN NOT NULL,
  is_transaction_success BOOLEAN NOT NULL,
  entry_function_id_str VARCHAR(100),
  block_height BIGINT NOT NULL,
  transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (
    transaction_version,
    event_account_address,
    event_creation_number,
    event_sequence_number
  )
);
CREATE INDEX ca_oa_ct_at_index on coin_activities (owner_address, coin_type, activity_type, amount);
CREATE INDEX ca_oa_igf_index on coin_activities (owner_address, is_gas_fee);
CREATE INDEX ca_ct_at_a_index on coin_activities (coin_type, activity_type, amount);
CREATE INDEX ca_ct_a_index on coin_activities (coin_type, amount);
CREATE INDEX ca_insat_index on coin_activities (inserted_at);