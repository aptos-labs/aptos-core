-- This file should undo anything in `up.sql`
ALTER TABLE collection_datas
DROP COLUMN table_handle;
ALTER TABLE current_collection_datas
DROP COLUMN table_handle;