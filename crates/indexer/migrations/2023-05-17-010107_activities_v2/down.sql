-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS token_activities_v2;
DROP INDEX IF EXISTS ta2_owner_type_index;
DROP INDEX IF EXISTS ta2_from_type_index;
DROP INDEX IF EXISTS ta2_to_type_index;
DROP INDEX IF EXISTS ta2_tid_index;
DROP INDEX IF EXISTS ta2_cid_index;
DROP INDEX IF EXISTS ta2_insat_index;