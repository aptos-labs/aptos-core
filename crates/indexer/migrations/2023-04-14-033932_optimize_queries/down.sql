-- This file should undo anything in `up.sql`
DROP VIEW IF EXISTS address_version_from_events;
DROP VIEW IF EXISTS address_version_from_move_resources;
DROP VIEW IF EXISTS current_collection_ownership_view;
DROP VIEW IF EXISTS num_active_delegator_per_pool;
DROP INDEX IF EXISTS curr_to_collection_hash_owner_index;