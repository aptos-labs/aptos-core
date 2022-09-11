-- Your SQL goes here

ALTER TABLE block_metadata_transactions
    ALTER COLUMN round TYPE uint_64;

ALTER TABLE block_metadata_transactions
    ALTER COLUMN epoch TYPE uint_64;

ALTER TABLE events
    ALTER COLUMN sequence_number TYPE uint_64;

ALTER TABLE processor_statuses
    ALTER COLUMN version TYPE uint_64;

ALTER TABLE transactions
    ALTER COLUMN version TYPE uint_64;

ALTER TABLE transactions
    ALTER COLUMN gas_used TYPE uint_64;

ALTER TABLE user_transactions
    ALTER COLUMN sequence_number TYPE uint_64;

ALTER TABLE user_transactions
    ALTER COLUMN max_gas_amount TYPE uint_64;

ALTER TABLE user_transactions
    ALTER COLUMN gas_unit_price TYPE uint_64;
