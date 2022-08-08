-- Your SQL goes here

ALTER TABLE block_metadata_transactions
    ALTER COLUMN round TYPE NUMERIC(78, 0);

ALTER TABLE block_metadata_transactions
    ALTER COLUMN epoch TYPE NUMERIC(78, 0);

ALTER TABLE events
    ALTER COLUMN sequence_number TYPE NUMERIC(78, 0);

ALTER TABLE processor_statuses
    ALTER COLUMN version TYPE NUMERIC(78, 0);

ALTER TABLE transactions
    ALTER COLUMN version TYPE NUMERIC(78, 0);

ALTER TABLE transactions
    ALTER COLUMN gas_used TYPE NUMERIC(78, 0);

ALTER TABLE user_transactions
    ALTER COLUMN sequence_number TYPE NUMERIC(78, 0);

ALTER TABLE user_transactions
    ALTER COLUMN max_gas_amount TYPE NUMERIC(78, 0);

ALTER TABLE user_transactions
    ALTER COLUMN gas_unit_price TYPE NUMERIC(78, 0);
