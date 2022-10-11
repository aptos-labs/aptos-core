-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS indexer_status;
DROP VIEW IF EXISTS events_view;
DROP VIEW IF EXISTS table_items_view;
DROP VIEW IF EXISTS transactions_view;
