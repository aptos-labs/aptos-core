-- This file should undo anything in `up.sql`
DROP VIEW IF EXISTS current_collection_ownership_v2_view;
DROP TABLE IF EXISTS current_token_v2_metadata;
ALTER TABLE token_datas_v2 DROP COLUMN IF EXISTS decimals;
ALTER TABLE current_token_datas_v2 DROP COLUMN IF EXISTS decimals;
ALTER TABLE token_ownerships_v2 DROP COLUMN IF EXISTS non_transferrable_by_owner;
ALTER TABLE current_token_ownerships_v2 DROP COLUMN IF EXISTS non_transferrable_by_owner;