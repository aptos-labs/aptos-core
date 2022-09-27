-- Your SQL goes here
-- tracks tokens per version
CREATE TABLE tokens (
  -- sha256 of creator + collection_name + name
  token_data_id_hash VARCHAR(64) NOT NULL,
  property_version NUMERIC NOT NULL,
  transaction_version BIGINT NOT NULL,
  creator_address VARCHAR(66) NOT NULL,
  collection_name VARCHAR(128) NOT NULL,
  name VARCHAR(128) NOT NULL,
  token_properties jsonb NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (
    token_data_id_hash,
    property_version,
    transaction_version
  )
);
CREATE INDEX token_crea_cn_name_index ON tokens (creator_address, collection_name, name);
CREATE INDEX token_insat_index ON tokens (inserted_at);
-- tracks who owns tokens at certain version
CREATE TABLE token_ownerships (
  -- sha256 of creator + collection_name + name
  token_data_id_hash VARCHAR(64) NOT NULL,
  property_version NUMERIC NOT NULL,
  transaction_version BIGINT NOT NULL,
  table_handle VARCHAR(66) NOT NULL,
  creator_address VARCHAR(66) NOT NULL,
  collection_name VARCHAR(128) NOT NULL,
  name VARCHAR(128) NOT NULL,
  owner_address VARCHAR(66),
  amount NUMERIC NOT NULL,
  table_type TEXT,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints  
  PRIMARY KEY (
    token_data_id_hash,
    property_version,
    transaction_version,
    table_handle
  )
);
CREATE INDEX to_owner_index ON token_ownerships (owner_address);
CREATE INDEX to_crea_cn_name_index ON token_ownerships (creator_address, collection_name, name);
CREATE INDEX to_insat_index ON token_ownerships (inserted_at);
-- tracks token metadata
CREATE TABLE token_datas (
  -- sha256 of creator + collection_name + name
  token_data_id_hash VARCHAR(64) NOT NULL,
  transaction_version BIGINT NOT NULL,
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
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (token_data_id_hash, transaction_version)
);
CREATE INDEX td_crea_cn_name_index ON token_datas (creator_address, collection_name, name);
CREATE INDEX td_insat_index ON token_datas (inserted_at);
-- tracks collection metadata
CREATE TABLE collection_datas (
  -- sha256 of creator + collection_name
  collection_data_id_hash VARCHAR(64) NOT NULL,
  transaction_version BIGINT NOT NULL,
  creator_address VARCHAR(66) NOT NULL,
  collection_name VARCHAR(128) NOT NULL,
  description TEXT NOT NULL,
  metadata_uri VARCHAR(512) NOT NULL,
  supply NUMERIC NOT NULL,
  maximum NUMERIC NOT NULL,
  maximum_mutable BOOLEAN NOT NULL,
  uri_mutable BOOLEAN NOT NULL,
  description_mutable BOOLEAN NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (collection_data_id_hash, transaction_version)
);
CREATE INDEX cd_crea_cn_index ON collection_datas (creator_address, collection_name);
CREATE INDEX cd_insat_index ON collection_datas (inserted_at);