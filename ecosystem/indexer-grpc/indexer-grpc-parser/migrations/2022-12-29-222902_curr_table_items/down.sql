-- This file should undo anything in `up.sql`
DROP VIEW IF EXISTS current_table_items_view;
DROP INDEX IF EXISTS cti_insat_index;
DROP TABLE IF EXISTS current_table_items;
ALTER TABLE events DROP COLUMN IF EXISTS event_index;
ALTER TABLE token_activities DROP COLUMN IF EXISTS event_index;
ALTER TABLE coin_activities DROP COLUMN IF EXISTS event_index;