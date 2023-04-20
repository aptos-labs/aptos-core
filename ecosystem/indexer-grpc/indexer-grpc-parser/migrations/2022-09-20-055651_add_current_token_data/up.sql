-- Your SQL goes here
-- tracks tokens in owner's tokenstore
CREATE TABLE current_token_ownerships (
  -- sha256 of creator + collection_name + name
  token_data_id_hash VARCHAR(64) NOT NULL,
  property_version NUMERIC NOT NULL,
  owner_address VARCHAR(66) NOT NULL,
  creator_address VARCHAR(66) NOT NULL,
  collection_name VARCHAR(128) NOT NULL,
  name VARCHAR(128) NOT NULL,
  amount NUMERIC NOT NULL,
  token_properties jsonb NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (
    token_data_id_hash,
    property_version,
    owner_address
  )
);
CREATE INDEX curr_to_crea_cn_name_index ON current_token_ownerships (creator_address, collection_name, name);
CREATE INDEX curr_to_owner_index ON current_token_ownerships (owner_address);
CREATE INDEX curr_to_insat_index ON current_token_ownerships (inserted_at);
-- tracks latest token metadata
CREATE TABLE current_token_datas (
  -- sha256 of creator + collection_name + name
  token_data_id_hash VARCHAR(64) UNIQUE PRIMARY KEY NOT NULL,
  creator_address VARCHAR(66) NOT NULL,
  collection_name VARCHAR(128) NOT NULL,
  name VARCHAR(128) NOT NULL,
  maximum NUMERIC NOT NULL,
  supply NUMERIC NOT NULL,
  largest_property_version NUMERIC NOT NULL,
  metadata_uri VARCHAR(512) NOT NULL,
  payee_address VARCHAR(66) NOT NULL,
  royalty_points_numerator NUMERIC NOT NULL,
  royalty_points_denominator NUMERIC NOT NULL,
  maximum_mutable BOOLEAN NOT NULL,
  uri_mutable BOOLEAN NOT NULL,
  description_mutable BOOLEAN NOT NULL,
  properties_mutable BOOLEAN NOT NULL,
  royalty_mutable BOOLEAN NOT NULL,
  default_properties jsonb NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX curr_td_crea_cn_name_index ON current_token_datas (creator_address, collection_name, name);
CREATE INDEX curr_td_insat_index ON current_token_datas (inserted_at);
-- tracks latest collection metadata
CREATE TABLE current_collection_datas (
  -- sha256 of creator + collection_name
  collection_data_id_hash VARCHAR(64) UNIQUE PRIMARY KEY NOT NULL,
  creator_address VARCHAR(66) NOT NULL,
  collection_name VARCHAR(128) NOT NULL,
  description TEXT NOT NULL,
  metadata_uri VARCHAR(512) NOT NULL,
  supply NUMERIC NOT NULL,
  maximum NUMERIC NOT NULL,
  maximum_mutable BOOLEAN NOT NULL,
  uri_mutable BOOLEAN NOT NULL,
  description_mutable BOOLEAN NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);
CREATE INDEX curr_cd_crea_cn_index ON current_collection_datas (creator_address, collection_name);
CREATE INDEX curr_cd_insat_index ON current_collection_datas (inserted_at);