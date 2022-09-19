-- Your SQL goes here
CREATE TABLE token_propertys
(
    token_id VARCHAR NOT NULL,
    previous_token_id VARCHAR NOT NULL,
    property_keys VARCHAR NOT NULL,
    property_values VARCHAR NOT NULL,
    property_types VARCHAR NOT NULL,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),

    -- Constraints
    PRIMARY KEY (token_id)
);