-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS at_version_index;
DROP INDEX IF EXISTS at_insat_index;
DROP TABLE IF EXISTS account_transactions;
ALTER TABLE objects
ALTER COLUMN owner_address DROP NOT NULL;
ALTER TABLE objects
ALTER COLUMN guid_creation_num DROP NOT NULL;
ALTER TABLE objects
ALTER COLUMN allow_ungated_transfer DROP NOT NULL;