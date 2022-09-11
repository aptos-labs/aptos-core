-- This file should undo anything in `up.sql`

ALTER TABLE block_metadata_transactions
    ALTER COLUMN round TYPE BIGINT;

ALTER TABLE block_metadata_transactions
    ALTER COLUMN epoch TYPE BIGINT;

ALTER TABLE events
    ALTER COLUMN sequence_number TYPE BIGINT;

ALTER TABLE processor_statuses
    ALTER COLUMN version TYPE BIGINT;

ALTER TABLE transactions
    ALTER COLUMN version TYPE BIGINT;

ALTER TABLE transactions
    ALTER COLUMN gas_used TYPE BIGINT;

ALTER TABLE user_transactions
    ALTER COLUMN sequence_number TYPE BIGINT;

ALTER TABLE user_transactions
    ALTER COLUMN max_gas_amount TYPE BIGINT;

ALTER TABLE user_transactions
    ALTER COLUMN gas_unit_price TYPE BIGINT;