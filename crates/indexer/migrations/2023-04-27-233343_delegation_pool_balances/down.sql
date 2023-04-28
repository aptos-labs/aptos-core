-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS delegated_staking_pool_balances;
DROP INDEX IF EXISTS dspb_insat_index;
ALTER TABLE current_delegator_balances DROP COLUMN IF EXISTS shares;