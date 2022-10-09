-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS coin_supply;
DROP INDEX IF EXISTS txn_v_index;
DROP INDEX IF EXISTS ev_tv_index;
DROP INDEX IF EXISTS wsc_v_index;
DROP INDEX IF EXISTS bmt_v_index;
DROP INDEX IF EXISTS bmt_ts_index;
ALTER TABLE coin_infos DROP COLUMN IF EXISTS supply_aggregator_table_handle,
  DROP COLUMN IF EXISTS supply_aggregator_table_key;