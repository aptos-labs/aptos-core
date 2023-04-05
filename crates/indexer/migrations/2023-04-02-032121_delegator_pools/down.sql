-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS delegated_staking_pools;
DROP INDEX IF EXISTS dsp_oa_index;
DROP INDEX IF EXISTS dsp_insat_index;
ALTER TABLE current_staking_pool_voter
DROP COLUMN IF EXISTS operator_address;