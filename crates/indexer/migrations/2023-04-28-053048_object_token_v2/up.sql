-- Your SQL goes here
-- objects, basically normalizing ObjectCore
CREATE TABLE IF NOT EXISTS objects (
  transaction_version BIGINT NOT NULL,
  write_set_change_index BIGINT NOT NULL,
  object_address VARCHAR(66) NOT NULL,
  owner_address VARCHAR(66),
  state_key_hash VARCHAR(66) NOT NULL,
  guid_creation_num NUMERIC,
  allow_ungated_transfer BOOLEAN,
  is_deleted BOOLEAN NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- constraints
  PRIMARY KEY (transaction_version, write_set_change_index)
);
CREATE INDEX IF NOT EXISTS o_owner_idx ON objects (owner_address);
CREATE INDEX IF NOT EXISTS o_object_skh_idx ON objects (object_address, state_key_hash);
CREATE INDEX IF NOT EXISTS o_skh_idx ON objects (state_key_hash);
CREATE INDEX IF NOT EXISTS o_insat_idx ON objects (inserted_at);
-- latest instance of objects
CREATE TABLE IF NOT EXISTS current_objects (
  object_address VARCHAR(66) UNIQUE PRIMARY KEY NOT NULL,
  owner_address VARCHAR(66) NOT NULL,
  state_key_hash VARCHAR(66) NOT NULL,
  allow_ungated_transfer BOOLEAN NOT NULL,
  last_guid_creation_num NUMERIC NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  is_deleted BOOLEAN NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS co_owner_idx ON current_objects (owner_address);
CREATE INDEX IF NOT EXISTS co_object_skh_idx ON current_objects (object_address, state_key_hash);
CREATE INDEX IF NOT EXISTS co_skh_idx ON current_objects (state_key_hash);
CREATE INDEX IF NOT EXISTS co_insat_idx ON current_objects (inserted_at);
-- Add this so that we can find resource groups by their state_key_hash
ALTER TABLE move_resources
ADD COLUMN IF NOT EXISTS state_key_hash VARCHAR(66) NOT NULL DEFAULT '';
-- NFT stuff
-- tracks who owns tokens
CREATE TABLE IF NOT EXISTS token_ownerships_v2 (
  transaction_version BIGINT NOT NULL,
  write_set_change_index BIGINT NOT NULL,
  token_data_id VARCHAR(66) NOT NULL,
  property_version_v1 NUMERIC NOT NULL,
  owner_address VARCHAR(66),
  storage_id VARCHAR(66) NOT NULL,
  amount NUMERIC NOT NULL,
  table_type_v1 VARCHAR(66),
  token_properties_mutated_v1 JSONB,
  is_soulbound_v2 BOOLEAN,
  token_standard VARCHAR(10) NOT NULL,
  is_fungible_v2 BOOLEAN,
  transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  PRIMARY KEY (transaction_version, write_set_change_index)
);
CREATE INDEX IF NOT EXISTS to2_id_index ON token_ownerships_v2 (token_data_id);
CREATE INDEX IF NOT EXISTS to2_owner_index ON token_ownerships_v2 (owner_address);
CREATE INDEX IF NOT EXISTS to2_insat_index ON token_ownerships_v2 (inserted_at);
CREATE TABLE IF NOT EXISTS current_token_ownerships_v2 (
  token_data_id VARCHAR(66) NOT NULL,
  property_version_v1 NUMERIC NOT NULL,
  owner_address VARCHAR(66) NOT NULL,
  storage_id VARCHAR(66) NOT NULL,
  amount NUMERIC NOT NULL,
  table_type_v1 VARCHAR(66),
  token_properties_mutated_v1 JSONB,
  is_soulbound_v2 BOOLEAN,
  token_standard VARCHAR(10) NOT NULL,
  is_fungible_v2 BOOLEAN,
  last_transaction_version BIGINT NOT NULL,
  last_transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  PRIMARY KEY (
    token_data_id,
    property_version_v1,
    owner_address,
    storage_id
  )
);
CREATE INDEX IF NOT EXISTS curr_to2_owner_index ON current_token_ownerships_v2 (owner_address);
CREATE INDEX IF NOT EXISTS curr_to2_wa_index ON current_token_ownerships_v2 (storage_id);
CREATE INDEX IF NOT EXISTS curr_to2_insat_index ON current_token_ownerships_v2 (inserted_at);
-- tracks collections
CREATE TABLE IF NOT EXISTS collections_v2 (
  transaction_version BIGINT NOT NULL,
  write_set_change_index BIGINT NOT NULL,
  collection_id VARCHAR(66) NOT NULL,
  creator_address VARCHAR(66) NOT NULL,
  collection_name VARCHAR(128) NOT NULL,
  description TEXT NOT NULL,
  uri VARCHAR(512) NOT NULL,
  current_supply NUMERIC NOT NULL,
  max_supply NUMERIC,
  total_minted_v2 NUMERIC,
  mutable_description BOOLEAN,
  mutable_uri BOOLEAN,
  table_handle_v1 VARCHAR(66),
  token_standard VARCHAR(10) NOT NULL,
  transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  PRIMARY KEY (transaction_version, write_set_change_index)
);
CREATE INDEX IF NOT EXISTS col2_id_index ON collections_v2 (collection_id);
CREATE INDEX IF NOT EXISTS col2_crea_cn_index ON collections_v2 (creator_address, collection_name);
CREATE INDEX IF NOT EXISTS col2_insat_index ON collections_v2 (inserted_at);
CREATE TABLE IF NOT EXISTS current_collections_v2 (
  collection_id VARCHAR(66) UNIQUE PRIMARY KEY NOT NULL,
  creator_address VARCHAR(66) NOT NULL,
  collection_name VARCHAR(128) NOT NULL,
  description TEXT NOT NULL,
  uri VARCHAR(512) NOT NULL,
  current_supply NUMERIC NOT NULL,
  max_supply NUMERIC,
  total_minted_v2 NUMERIC,
  mutable_description BOOLEAN,
  mutable_uri BOOLEAN,
  table_handle_v1 VARCHAR(66),
  token_standard VARCHAR(10) NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  last_transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS cur_col2_crea_cn_index ON current_collections_v2 (creator_address, collection_name);
CREATE INDEX IF NOT EXISTS cur_col2_insat_index ON current_collections_v2 (inserted_at);
-- tracks token metadata
CREATE TABLE IF NOT EXISTS token_datas_v2 (
  transaction_version BIGINT NOT NULL,
  write_set_change_index BIGINT NOT NULL,
  token_data_id VARCHAR(66) NOT NULL,
  collection_id VARCHAR(66) NOT NULL,
  token_name VARCHAR(128) NOT NULL,
  maximum NUMERIC,
  supply NUMERIC NOT NULL,
  largest_property_version_v1 NUMERIC,
  token_uri VARCHAR(512) NOT NULL,
  token_properties JSONB NOT NULL,
  description TEXT NOT NULL,
  token_standard VARCHAR(10) NOT NULL,
  is_fungible_v2 BOOLEAN,
  transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  PRIMARY KEY (transaction_version, write_set_change_index)
);
CREATE INDEX IF NOT EXISTS td2_id_index ON token_datas_v2 (token_data_id);
CREATE INDEX IF NOT EXISTS td2_cid_name_index ON token_datas_v2 (collection_id, token_name);
CREATE INDEX IF NOT EXISTS td2_insat_index ON token_datas_v2 (inserted_at);
CREATE TABLE IF NOT EXISTS current_token_datas_v2 (
  token_data_id VARCHAR(66) UNIQUE PRIMARY KEY NOT NULL,
  collection_id VARCHAR(66) NOT NULL,
  token_name VARCHAR(128) NOT NULL,
  maximum NUMERIC,
  supply NUMERIC NOT NULL,
  largest_property_version_v1 NUMERIC,
  token_uri VARCHAR(512) NOT NULL,
  description TEXT NOT NULL,
  token_properties JSONB NOT NULL,
  token_standard VARCHAR(10) NOT NULL,
  is_fungible_v2 BOOLEAN,
  last_transaction_version BIGINT NOT NULL,
  last_transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS cur_td2_cid_name_index ON current_token_datas_v2 (collection_id, token_name);
CREATE INDEX IF NOT EXISTS cur_td2_insat_index ON current_token_datas_v2 (inserted_at);
-- Add ID (with 0x prefix)
ALTER TABLE current_token_pending_claims
ADD COLUMN IF NOT EXISTS token_data_id VARCHAR(66) NOT NULL DEFAULT '';
ALTER TABLE current_token_pending_claims
ADD COLUMN IF NOT EXISTS collection_id VARCHAR(66) NOT NULL DEFAULT '';