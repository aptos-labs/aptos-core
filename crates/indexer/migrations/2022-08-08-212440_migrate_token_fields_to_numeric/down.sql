-- This file should undo anything in `up.sql`

ALTER TABLE metadatas
    ALTER COLUMN seller_fee_basis_points TYPE BIGINT;

ALTER TABLE collections
    ALTER COLUMN max_amount TYPE varchar;

ALTER TABLE ownerships
    ALTER COLUMN amount TYPE BIGINT;

ALTER TABLE token_activities
    ALTER COLUMN sequence_number TYPE BIGINT;

ALTER TABLE token_datas
    ALTER COLUMN supply TYPE BIGINT;

ALTER TABLE token_datas
    ALTER COLUMN royalty_points_denominator TYPE BIGINT;

ALTER TABLE token_datas
    ALTER COLUMN royalty_points_numerator TYPE BIGINT;

ALTER TABLE token_datas
    ALTER COLUMN max_amount TYPE varchar;
