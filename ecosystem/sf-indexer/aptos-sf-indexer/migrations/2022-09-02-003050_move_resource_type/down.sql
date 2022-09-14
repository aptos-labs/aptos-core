-- This file should undo anything in `up.sql`
ALTER TABLE move_resources DROP COLUMN type_str;
ALTER TABLE events DROP COLUMN type_str;