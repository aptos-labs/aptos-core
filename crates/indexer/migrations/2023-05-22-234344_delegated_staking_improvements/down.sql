-- This file should undo anything in `up.sql`
ALTER TABLE current_delegated_staking_pool_balances DROP COLUMN IF EXISTS operator_commission_percentage,
  DROP COLUMN IF EXISTS inactive_table_handle,
  DROP COLUMN IF EXISTS active_table_handle;
DROP INDEX IF EXISTS cdspb_inactive_index;
ALTER TABLE delegated_staking_pool_balances DROP COLUMN IF EXISTS operator_commission_percentage,
  DROP COLUMN IF EXISTS inactive_table_handle,
  DROP COLUMN IF EXISTS active_table_handle;
ALTER TABLE current_delegator_balances DROP COLUMN IF EXISTS parent_table_handle;
ALTER TABLE current_delegator_balances DROP CONSTRAINT current_delegator_balances_pkey;
ALTER TABLE current_delegator_balances
ADD CONSTRAINT current_delegator_balances_pkey PRIMARY KEY (
    delegator_address,
    pool_address,
    pool_type
  );
CREATE OR REPLACE VIEW num_active_delegator_per_pool AS
SELECT pool_address,
  COUNT(DISTINCT delegator_address) AS num_active_delegator
FROM current_delegator_balances
WHERE shares > 0
GROUP BY 1;
DROP VIEW IF EXISTS delegator_distinct_pool;
DROP VIEW IF EXISTS address_events_summary;