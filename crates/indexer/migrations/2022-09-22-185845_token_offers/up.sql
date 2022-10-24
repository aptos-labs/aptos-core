-- Your SQL goes here
ALTER TABLE current_token_ownerships
ADD COLUMN collection_data_id_hash VARCHAR(64) NOT NULL,
  ADD COLUMN table_type TEXT NOT NULL;
ALTER TABLE current_token_datas
ADD COLUMN collection_data_id_hash VARCHAR(64) NOT NULL;
ALTER TABLE token_datas
ADD COLUMN collection_data_id_hash VARCHAR(64) NOT NULL;
ALTER TABLE tokens
ADD COLUMN collection_data_id_hash VARCHAR(64) NOT NULL;
ALTER TABLE token_ownerships
ADD COLUMN collection_data_id_hash VARCHAR(64) NOT NULL;
-- add indices for current ownership to speed up queries
CREATE INDEX curr_to_owner_tt_am_index ON current_token_ownerships (owner_address, table_type, amount);
-- tracks all token activities
CREATE TABLE token_activities (
  transaction_version BIGINT NOT NULL,
  event_account_address VARCHAR(66) NOT NULL,
  event_creation_number BIGINT NOT NULL,
  event_sequence_number BIGINT NOT NULL,
  collection_data_id_hash VARCHAR(64) NOT NULL,
  token_data_id_hash VARCHAR(64) NOT NULL,
  property_version NUMERIC NOT NULL,
  creator_address VARCHAR(66) NOT NULL,
  collection_name VARCHAR(128) NOT NULL,
  name VARCHAR(128) NOT NULL,
  transfer_type VARCHAR(50) NOT NULL,
  from_address VARCHAR(66),
  to_address VARCHAR(66),
  token_amount NUMERIC NOT NULL,
  coin_type TEXT,
  coin_amount NUMERIC,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (
    transaction_version,
    event_account_address,
    event_creation_number,
    event_sequence_number
  )
);
CREATE INDEX ta_from_ttyp_index ON token_activities (from_address, transfer_type);
CREATE INDEX ta_to_ttyp_index ON token_activities (to_address, transfer_type);
CREATE INDEX ta_addr_coll_name_pv_index ON token_activities (
  creator_address,
  collection_name,
  name,
  property_version
);
CREATE INDEX ta_tdih_pv_index ON token_activities (token_data_id_hash, property_version);
CREATE INDEX ta_version_index ON token_activities (transaction_version);
CREATE INDEX ta_insat_index ON token_activities (inserted_at);
-- Tracks current pending claims
CREATE TABLE current_token_pending_claims (
  token_data_id_hash VARCHAR(64) NOT NULL,
  property_version NUMERIC NOT NULL,
  from_address VARCHAR(66) NOT NULL,
  to_address VARCHAR(66) NOT NULL,
  collection_data_id_hash VARCHAR(64) NOT NULL,
  creator_address VARCHAR(66) NOT NULL,
  collection_name VARCHAR(128) NOT NULL,
  name VARCHAR(128) NOT NULL,
  -- 0 means either claimed or canceled
  amount NUMERIC NOT NULL,
  table_handle VARCHAR(66) NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (
    -- This is basically the token offer id
    token_data_id_hash,
    property_version,
    from_address,
    to_address
  )
);
CREATE INDEX ctpc_th_index ON current_token_pending_claims (table_handle);
CREATE INDEX ctpc_from_am_index ON current_token_pending_claims (from_address, amount);
CREATE INDEX ctpc_to_am_index ON current_token_pending_claims (to_address, amount);
CREATE INDEX ctpc_insat_index ON current_token_pending_claims (inserted_at);