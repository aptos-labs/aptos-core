-- Your SQL goes here
-- This is needed to improve performance when querying an account with a large number of transactions
CREATE INDEX IF NOT EXISTS mr_ver_index ON move_resources(transaction_version DESC);
-- These are needed b/c for some reason we're getting build errors when setting
-- type field with a length limit
ALTER TABLE signatures
ALTER COLUMN type TYPE VARCHAR;
ALTER TABLE token_activities_v2
ALTER COLUMN type TYPE VARCHAR;
DROP VIEW IF EXISTS transactions_view;
ALTER TABLE transactions
ALTER COLUMN type TYPE VARCHAR;
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