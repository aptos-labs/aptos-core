-- Your SQL goes here
CREATE TABLE token_datas
(
    token_data_id VARCHAR NOT NULL,
    creator VARCHAR NOT NULL,
    collection VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    description VARCHAR NOT NULL,
    max_amount VARCHAR NOT NULL,
    supply bigint NOT NULL,
    uri VARCHAR NOT NULL,
    royalty_payee_address VARCHAR NOT NULL,
    royalty_points_denominator bigint NOT NULL,
    royalty_points_numerator bigint NOT NULL,
    mutability_config VARCHAR NOT NULL,
    property_keys VARCHAR NOT NULL,
    property_values VARCHAR NOT NULL,
    property_types VARCHAR NOT NULL,
    minted_at TIMESTAMP NOT NULL,
    last_minted_at TIMESTAMP NOT NULL,

    inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),

    -- Constraints
    PRIMARY KEY (token_data_id)
);
