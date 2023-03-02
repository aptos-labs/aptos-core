-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS coin_infos;
DROP TABLE IF EXISTS coin_balances;
DROP TABLE IF EXISTS current_coin_balances;
DROP TABLE IF EXISTS coin_activities;
ALTER TABLE token_activities
DROP COLUMN IF EXISTS transaction_timestamp;
ALTER TABLE current_token_pending_claims
DROP COLUMN IF EXISTS last_transaction_timestamp;
ALTER TABLE current_token_ownerships
DROP COLUMN IF EXISTS last_transaction_timestamp;
ALTER TABLE current_token_datas
DROP COLUMN IF EXISTS last_transaction_timestamp;
ALTER TABLE current_collection_datas
DROP COLUMN IF EXISTS last_transaction_timestamp;
ALTER TABLE tokens
DROP COLUMN IF EXISTS transaction_timestamp;
ALTER TABLE token_ownerships
DROP COLUMN IF EXISTS transaction_timestamp;
ALTER TABLE token_datas
DROP COLUMN IF EXISTS transaction_timestamp;
ALTER TABLE collection_datas
DROP COLUMN IF EXISTS transaction_timestamp;
DROP VIEW IF EXISTS move_resources_view;