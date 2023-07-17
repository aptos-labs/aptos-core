-- This file should undo anything in `up.sql`
DROP TABLE IF EXISTS delegated_staking_pool_balances;
DROP TABLE IF EXISTS current_delegated_staking_pool_balances;
DROP INDEX IF EXISTS dspb_insat_index;
ALTER TABLE current_delegator_balances
ADD COLUMN IF NOT EXISTS amount NUMERIC NOT NULL DEFAULT 0;
-- need this for delegation staking, changing to amount
CREATE OR REPLACE VIEW num_active_delegator_per_pool AS
SELECT pool_address,
  COUNT(DISTINCT delegator_address) AS num_active_delegator
FROM current_delegator_balances
WHERE amount > 0
GROUP BY 1;
ALTER TABLE current_delegator_balances DROP COLUMN IF EXISTS shares;