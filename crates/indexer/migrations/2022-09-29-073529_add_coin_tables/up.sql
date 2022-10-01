-- Your SQL goes here
-- coin infos. Only first transaction matters
CREATE TABLE coin_infos (
  -- creator_address::name::symbol
  -- Max length should actually only be 66 + 32 + 10 = 108 but we didn't enforce these in the beginning
  coin_type VARCHAR(256) UNIQUE PRIMARY KEY NOT NULL,
  -- transaction version where coin info was first defined
  transaction_version_created BIGINT NOT NULL,
  creator_address VARCHAR(66) NOT NULL,
  name VARCHAR(32) NOT NULL,
  symbol VARCHAR(10) NOT NULL,
  decimals INT NOT NULL,
  supply NUMERIC NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX ci_ca_name_symbol_index on coin_infos (creator_address, name, symbol);
CREATE INDEX ci_insat_index ON coin_infos (inserted_at);
-- current coin owned by user
CREATE TABLE coin_balances (
  transaction_version BIGINT NOT NULL,
  owner_address VARCHAR(66) NOT NULL,
  -- creator_address::name::symbol
  coin_type VARCHAR(256) NOT NULL,
  amount NUMERIC NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (transaction_version, owner_address, coin_type)
);
CREATE INDEX cb_oa_ct_index on coin_balances (owner_address, coin_type);
CREATE INDEX cb_ct_a_index on coin_balances (coin_type, amount);
CREATE INDEX cb_insat_index ON coin_balances (inserted_at);
-- current coin owned by user
CREATE TABLE current_coin_balances (
  owner_address VARCHAR(66) NOT NULL,
  -- creator_address::name::symbol
  coin_type VARCHAR(256) NOT NULL,
  amount NUMERIC NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (owner_address, coin_type)
);
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
  coin_type VARCHAR(256) NOT NULL,
  amount NUMERIC NOT NULL,
  activity_type VARCHAR(200) NOT NULL,
  is_gas_fee BOOLEAN NOT NULL,
  is_transaction_success BOOLEAN NOT NULL,
  entry_function_id_str VARCHAR(100),
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