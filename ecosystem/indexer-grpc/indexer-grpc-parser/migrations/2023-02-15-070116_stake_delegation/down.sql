-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS delegated_staking_activities;
DROP INDEX IF EXISTS dsa_pa_da_index;
DROP INDEX IF EXISTS dsa_insat_index;
DROP TABLE IF EXISTS current_delegator_balances;
DROP INDEX IF EXISTS cdb_insat_index;