-- Your SQL goes here
CREATE TABLE metadatas
(
    token_id VARCHAR NOT NULL,
    name VARCHAR,
    symbol VARCHAR,
    seller_fee_basis_points BIGINT,
    description VARCHAR,
    image VARCHAR NOT NULL,
    external_url VARCHAR,
    animation_url VARCHAR,
    attributes jsonb,
    properties jsonb,

    last_updated_at TIMESTAMP NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),
    -- Constraints
    PRIMARY KEY (token_id)
);
