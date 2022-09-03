-- Your SQL goes here
ALTER TABLE user_transactions
ADD COLUMN entry_function_id_str text NOT NULL;
CREATE TABLE tokens (
  creator_address VARCHAR(100) NOT NULL,
  collection_name TEXT NOT NULL,
  name TEXT NOT NULL,
  property_version NUMERIC NOT NULL,
  transaction_version BIGINT NOT NULL,
  token_properties jsonb NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (
    creator_address,
    collection_name,
    name,
    property_version,
    transaction_version
  )
);
CREATE TABLE token_ownerships (
  creator_address VARCHAR(100) NOT NULL,
  collection_name TEXT NOT NULL,
  name TEXT NOT NULL,
  property_version NUMERIC NOT NULL,
  transaction_version BIGINT NOT NULL,
  owner_address VARCHAR(100),
  amount NUMERIC NOT NULL,
  table_handle VARCHAR(255) NOT NULL,
  table_type TEXT,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints  
  PRIMARY KEY (
    creator_address,
    collection_name,
    name,
    property_version,
    transaction_version,
    table_handle
  )
);
CREATE INDEX to_creator_coll_name_pv_tv ON token_ownerships (
  creator_address,
  collection_name,
  name,
  property_version,
  transaction_version
);
CREATE INDEX to_owner ON token_ownerships (owner_address);
CREATE TABLE token_datas (
  creator_address VARCHAR(100) NOT NULL,
  collection_name TEXT NOT NULL,
  name TEXT NOT NULL,
  transaction_version BIGINT NOT NULL,
  maximum NUMERIC NOT NULL,
  supply NUMERIC NOT NULL,
  largest_property_version NUMERIC NOT NULL,
  metadata_uri TEXT NOT NULL,
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
  PRIMARY KEY (
    creator_address,
    collection_name,
    name,
    transaction_version
  )
);
CREATE INDEX td_creator_coll_name ON token_datas (creator_address, collection_name, name);
CREATE TABLE collection_datas (
  creator_address VARCHAR(100) NOT NULL,
  collection_name TEXT NOT NULL,
  description TEXT NOT NULL,
  transaction_version BIGINT NOT NULL,
  metadata_uri TEXT NOT NULL,
  supply NUMERIC NOT NULL,
  maximum NUMERIC NOT NULL,
  maximum_mutable BOOLEAN NOT NULL,
  uri_mutable BOOLEAN NOT NULL,
  description_mutable BOOLEAN NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (
    creator_address,
    collection_name,
    transaction_version
  )
);
CREATE INDEX td_creator_coll ON collection_datas (creator_address, collection_name);