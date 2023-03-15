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
  -- Constraints
  PRIMARY KEY (
    table_handle,
    key_hash
  )
);
ALTER TABLE events
ADD COLUMN event_index BIGINT;
ALTER TABLE token_activities
ADD COLUMN event_index BIGINT;
ALTER TABLE coin_activities
ADD COLUMN event_index BIGINT;