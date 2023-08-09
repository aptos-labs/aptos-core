-- Your SQL goes here
-- need this for getting NFTs grouped by collections
create or replace view current_collection_ownership_v2_view as
select owner_address,
  b.collection_id,
  MAX(a.last_transaction_version) as last_transaction_version,
  COUNT(distinct a.token_data_id) as distinct_tokens
from current_token_ownerships_v2 a
  join current_token_datas_v2 b on a.token_data_id = b.token_data_id
where a.amount > 0
group by 1,
  2;
-- create table for all structs in token object core
CREATE TABLE IF NOT EXISTS current_token_v2_metadata (
  object_address VARCHAR(66) NOT NULL,
  resource_type VARCHAR(128) NOT NULL,
  data jsonb NOT NULL,
  state_key_hash VARCHAR(66) NOT NULL,
  last_transaction_version BIGINT NOT NULL,
  inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
  -- constraints
  PRIMARY KEY (object_address, resource_type)
);
-- create table for all structs in token object core
ALTER TABLE token_datas_v2
ADD COLUMN IF NOT EXISTS decimals BIGINT NOT NULL DEFAULT 0;
ALTER TABLE current_token_datas_v2
ADD COLUMN IF NOT EXISTS decimals BIGINT NOT NULL DEFAULT 0;
ALTER TABLE token_ownerships_v2
ADD COLUMN IF NOT EXISTS non_transferrable_by_owner BOOLEAN;
ALTER TABLE current_token_ownerships_v2
ADD COLUMN IF NOT EXISTS non_transferrable_by_owner BOOLEAN;
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