-- Your SQL goes here

ALTER TABLE metadatas
    ALTER COLUMN seller_fee_basis_points TYPE uint_64;

ALTER TABLE collections
    ALTER COLUMN max_amount TYPE uint_64 USING (max_amount::uint_64);

ALTER TABLE ownerships
    ALTER COLUMN amount TYPE uint_64;

ALTER TABLE token_activities
    ALTER COLUMN sequence_number TYPE uint_64;

ALTER TABLE token_datas
    ALTER COLUMN supply TYPE uint_64;

ALTER TABLE token_datas
    ALTER COLUMN royalty_points_denominator TYPE uint_64;

ALTER TABLE token_datas
    ALTER COLUMN royalty_points_numerator TYPE uint_64;

ALTER TABLE token_datas
    ALTER COLUMN max_amount TYPE uint_64 USING (max_amount::uint_64);
