-- This file should undo anything in `up.sql`
ALTER TABLE current_token_ownerships DROP COLUMN collection_data_id_hash,
  DROP COLUMN table_type;
ALTER TABLE current_token_datas DROP COLUMN collection_data_id_hash;
ALTER TABLE token_datas DROP COLUMN collection_data_id_hash;
ALTER TABLE tokens DROP COLUMN collection_data_id_hash;
ALTER TABLE token_ownerships DROP COLUMN collection_data_id_hash;
DROP INDEX IF EXISTS curr_to_owner_tt_am_index;
DROP TABLE IF EXISTS token_activities;
DROP TABLE IF EXISTS current_token_pending_claims;