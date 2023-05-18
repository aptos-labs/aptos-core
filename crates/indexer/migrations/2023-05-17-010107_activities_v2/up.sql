-- Your SQL goes here
CREATE TABLE IF NOT EXISTS token_activities_v2 (
  transaction_version BIGINT NOT NULL,
  event_index BIGINT NOT NULL,
  event_account_address VARCHAR(66) NOT NULL,
  token_data_id VARCHAR(66) NOT NULL,
  property_version_v1 NUMERIC NOT NULL,
  type VARCHAR(50) NOT NULL,
  from_address VARCHAR(66),
  to_address VARCHAR(66),
  token_amount NUMERIC NOT NULL,
  before_value TEXT,
  after_value TEXT,
  entry_function_id_str VARCHAR(100),
  token_standard VARCHAR(10) NOT NULL,
  is_fungible_v2 BOOLEAN,
  transaction_timestamp TIMESTAMP NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  PRIMARY KEY (transaction_version, event_index)
);
CREATE INDEX IF NOT EXISTS ta2_owner_type_index ON token_activities_v2 (event_account_address, type);
CREATE INDEX IF NOT EXISTS ta2_from_type_index ON token_activities_v2 (from_address, type);
CREATE INDEX IF NOT EXISTS ta2_to_type_index ON token_activities_v2 (to_address, type);
CREATE INDEX IF NOT EXISTS ta2_tid_index ON token_activities_v2 (token_data_id);
CREATE INDEX IF NOT EXISTS ta2_insat_index ON token_activities_v2 (inserted_at);