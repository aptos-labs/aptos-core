-- Your SQL goes here
-- manually toggle indexer status on/off
CREATE TABLE indexer_status (
  db VARCHAR(50) UNIQUE PRIMARY KEY NOT NULL,
  is_indexer_up BOOLEAN NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW()
);
-- Create event view to avoid large jsonb
CREATE VIEW events_view AS
SELECT sequence_number,
  creation_number,
  account_address,
  transaction_version,
  transaction_block_height,
  "type",
  "data"#>>'{}' AS json_data,
  inserted_at
FROM events;
-- Create table_items view to avoid large jsonb
CREATE VIEW table_items_view AS
SELECT "key",
  transaction_version,
  write_set_change_index,
  transaction_block_height,
  table_handle,
  decoded_key#>>'{}' AS json_decoded_key,
  decoded_value#>>'{}' AS json_decoded_value,
  is_deleted,
  inserted_at
FROM table_items;
-- Create transactions view to avoid large jsonb
CREATE VIEW transactions_view AS
SELECT "version",
  block_height,
  "hash",
  "type",
  payload#>>'{}' AS json_payload,
  state_change_hash,
  event_root_hash,
  state_checkpoint_hash,
  gas_used,
  success,
  vm_status,
  accumulator_root_hash,
  num_events,
  num_write_set_changes,
  inserted_at
FROM transactions;