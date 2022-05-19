-- Your SQL goes here
CREATE TABLE ownerships
(
    token_id VARCHAR,
    owner VARCHAR,
    amount NUMERIC,
    updated_at TIMESTAMP NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),

    PRIMARY KEY (token_id, owner)
);
