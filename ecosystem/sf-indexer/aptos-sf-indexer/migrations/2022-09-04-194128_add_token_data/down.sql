-- This file should undo anything in `up.sql`
ALTER TABLE user_transactions DROP COLUMN entry_function_id_str;
DROP TABLE tokens;
DROP TABLE token_ownerships;
DROP TABLE token_datas;
DROP TABLE collection_datas;