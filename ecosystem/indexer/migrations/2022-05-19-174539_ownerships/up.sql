-- Your SQL goes here
CREATE TABLE ownerships
(
    ownership_id VARCHAR NOT NULL,
    token_id VARCHAR,
    owner VARCHAR,
    amount BIGINT NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    inserted_at TIMESTAMP NOT NULL DEFAULT NOW(),

    PRIMARY KEY (ownership_id)
);
