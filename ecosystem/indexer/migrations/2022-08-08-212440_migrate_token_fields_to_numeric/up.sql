-- Your SQL goes here

ALTER TABLE metadatas
    ALTER COLUMN seller_fee_basis_points TYPE NUMERIC(78, 0);

ALTER TABLE ownerships
    ALTER COLUMN amount TYPE NUMERIC(78, 0);

ALTER TABLE token_activities
    ALTER COLUMN sequence_number TYPE NUMERIC(78, 0);

ALTER TABLE token_datas
    ALTER COLUMN supply TYPE NUMERIC(78, 0);

ALTER TABLE token_datas
    ALTER COLUMN royalty_points_denominator TYPE NUMERIC(78, 0);

ALTER TABLE token_datas
    ALTER COLUMN royalty_points_numerator TYPE NUMERIC(78, 0);
