-- Your SQL goes here
CREATE TABLE current_table_items (
  table_handle VARCHAR(66) NOT NULL,
  -- Hash of the key for pk since key is unbounded
  key_hash VARCHAR(64) NOT NULL,
  key text NOT NULL,
  decoded_key jsonb NOT NULL,
  decoded_value jsonb,
  is_deleted BOOLEAN NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- Constraints
  PRIMARY KEY (
    table_handle,
    key_hash
  )
);
CREATE INDEX cti_insat_index ON current_table_items (inserted_at);
-- Create view to avoid large jsonb in bigquery
CREATE VIEW current_table_items_view AS
SELECT "key",
  table_handle,
  key_hash,
  decoded_key#>>'{}' AS json_decoded_key,
  decoded_value#>>'{}' AS json_decoded_value,
  is_deleted,
  last_transaction_version,
  inserted_at
FROM current_table_items;
ALTER TABLE events
ADD COLUMN event_index BIGINT;
ALTER TABLE token_activities
ADD COLUMN event_index BIGINT;
ALTER TABLE coin_activities
ADD COLUMN event_index BIGINT;