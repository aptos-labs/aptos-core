-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS coin_supply;
DROP INDEX IF EXISTS cs_ct_tv_index;
DROP INDEX IF EXISTS cs_epoch_index;
ALTER TABLE coin_infos DROP COLUMN IF EXISTS supply_aggregator_table_handle,
  DROP COLUMN IF EXISTS supply_aggregator_table_key;
ALTER TABLE token_datas DROP COLUMN IF EXISTS description;
ALTER TABLE current_token_datas DROP COLUMN IF EXISTS description;
ALTER TABLE user_transactions DROP COLUMN IF EXISTS epoch;
ALTER TABLE transactions DROP COLUMN IF EXISTS epoch;
DROP INDEX IF EXISTS ut_epoch_index;
DROP INDEX IF EXISTS txn_epoch_index;