-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS ans_tn_index;
ALTER TABLE current_ans_lookup DROP COLUMN IF EXISTS token_name;
DROP INDEX IF EXISTS pv_pi_va_index;
DROP INDEX IF EXISTS pv_va_index;
DROP INDEX IF EXISTS pv_spa_index;
DROP INDEX IF EXISTS pv_ia_index;
DROP TABLE IF EXISTS proposal_votes;